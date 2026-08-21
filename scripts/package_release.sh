#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VERSION="${1:-0.1.0}"
if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]]; then
  printf '%s\n' "REFUSED_RELEASE_VERSION value=$VERSION" >&2
  exit 2
fi
if [[ "$(uname -s)" != "Darwin" || "$(uname -m)" != "arm64" ]]; then
  printf '%s\n' 'REFUSED_RELEASE_HOST requires=macOS-arm64' >&2
  exit 2
fi
if [[ -n "$(git status --porcelain=v1 --untracked-files=all)" ]]; then
  printf '%s\n' 'REFUSED_RELEASE_DIRTY_WORKTREE' >&2
  exit 2
fi

PACKAGE_VERSION="$(node -p "require('./desktop/package.json').version")"
TAURI_VERSION="$(python3 -c 'import json; print(json.load(open("desktop/src-tauri/tauri.conf.json"))["version"])')"
CARGO_VERSION="$(python3 -c 'import pathlib, re; print(re.search(r"^version = \"([^\"]+)\"", pathlib.Path("desktop/src-tauri/Cargo.toml").read_text(), re.M).group(1))')"
if [[ "$VERSION" != "$PACKAGE_VERSION" || "$VERSION" != "$TAURI_VERSION" || "$VERSION" != "$CARGO_VERSION" ]]; then
  printf '%s\n' \
    "REFUSED_RELEASE_VERSION_DRIFT requested=$VERSION package=$PACKAGE_VERSION tauri=$TAURI_VERSION cargo=$CARGO_VERSION" >&2
  exit 2
fi

export CARGO_INCREMENTAL=0
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--remap-path-prefix=$ROOT=/build/nemesis --remap-path-prefix=$HOME=/build/home"
if [[ "$(cargo about --version)" != "cargo-about 0.9.2" ]]; then
  printf '%s\n' 'REFUSED_RELEASE_CARGO_ABOUT requires=0.9.2' >&2
  exit 2
fi


python3 scripts/verify_release_contract.py
NEMESIS_BUILD=release ./scripts/build_ada.sh
cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
cargo build --manifest-path runtime/Cargo.toml --workspace --release
npm --prefix desktop ci
LICENSE_DIR="$ROOT/build/release"
mkdir -p "$LICENSE_DIR"
cargo about generate \
  --manifest-path runtime/Cargo.toml \
  --workspace \
  --config runtime/about.toml \
  --format json \
  --output-file "$LICENSE_DIR/runtime-licenses.json" \
  --locked \
  --fail
cargo about generate \
  --manifest-path desktop/src-tauri/Cargo.toml \
  --config runtime/about.toml \
  --format json \
  --output-file "$LICENSE_DIR/desktop-licenses.json" \
  --locked \
  --fail
python3 scripts/generate_third_party_notices.py \
  --cargo-json "$LICENSE_DIR/runtime-licenses.json" \
  --cargo-json "$LICENSE_DIR/desktop-licenses.json" \
  --desktop desktop \
  --output "$LICENSE_DIR/THIRD_PARTY_NOTICES.txt"
npm --prefix desktop test
npm --prefix desktop run build
npm --prefix desktop run tauri -- \
  build \
  --config src-tauri/tauri.release.conf.json \
  --bundles app

APP="$ROOT/desktop/src-tauri/target/release/bundle/macos/NEMESIS Desktop.app"
BINARY="$APP/Contents/MacOS/nemesis-desktop"
INFO_PLIST="$APP/Contents/Info.plist"
if [[ ! -d "$APP" || ! -x "$BINARY" ]]; then
  printf '%s\n' "FAIL_RELEASE_APP_MISSING path=$APP" >&2
  exit 1
fi
for binary in \
  nemesis_core_daemon \
  nemesis-deterministic-worker \
  nemesis-lane-create \
  nemesis-signer \
  nemesis-verify \
  nemesis-worker-runner
do
  if [[ ! -x "$APP/Contents/Resources/bin/$binary" ]]; then
    printf '%s\n' "FAIL_RELEASE_RESOURCE_MISSING name=$binary" >&2
    exit 1
  fi
done
RELEASE_BINARIES=(
  "$BINARY"
  "$APP/Contents/Resources/bin/nemesis_core_daemon"
  "$APP/Contents/Resources/bin/nemesis-deterministic-worker"
  "$APP/Contents/Resources/bin/nemesis-lane-create"
  "$APP/Contents/Resources/bin/nemesis-signer"
  "$APP/Contents/Resources/bin/nemesis-verify"
  "$APP/Contents/Resources/bin/nemesis-worker-runner"
)
python3 scripts/sanitize_macho.py "${RELEASE_BINARIES[@]}"
for release_binary in "${RELEASE_BINARIES[@]}"; do
  strip -x "$release_binary"
done
python3 scripts/check_release_leaks.py "${RELEASE_BINARIES[@]}"


plutil -lint "$INFO_PLIST"
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"
ARCHITECTURES="$(lipo -archs "$BINARY")"
if [[ "$ARCHITECTURES" != "arm64" ]]; then
  printf '%s\n' "FAIL_RELEASE_ARCHITECTURE observed=$ARCHITECTURES" >&2
  exit 1
fi

COMMIT="$(git rev-parse HEAD)"
TREE="$(git rev-parse HEAD^{tree})"
OUTPUT="$ROOT/release/v$VERSION"
ASSET_NAME="NEMESIS-Desktop-v$VERSION-macos-arm64.zip"
rm -rf "$OUTPUT"
mkdir -p "$OUTPUT"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$OUTPUT/$ASSET_NAME"
python3 scripts/release_manifest.py \
  --asset "$OUTPUT/$ASSET_NAME" \
  --version "$VERSION" \
  --commit "$COMMIT" \
  --tree "$TREE" \
  --architecture arm64 \
  --output-dir "$OUTPUT"
(
  cd "$OUTPUT"
  shasum -a 256 -c SHA256SUMS
)
printf '%s\n' \
  "PASS_RELEASE_PACKAGE version=$VERSION commit=$COMMIT asset=$ASSET_NAME signing=ad-hoc notarized=false"
