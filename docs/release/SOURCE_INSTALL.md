# NEMESIS Desktop — Supported Distribution Contract (Source Install)

Effective 2026-08-22, per architect authority recorded in
`receipts/production-readiness-20260821/trust-surface/TS_CONTINUATION_CONTRACT_2026-08-22.md`
(sha256 `920900f3a7d37a9a3e1d51541997070789a0e1e7bddecf31808076c67a00d3f9`):
Developer ID signing, Apple Developer Program membership, notarization, and
Gatekeeper acceptance are **not requirements** of this product. The supported
distribution channels are:

1. **Source install** — clone the repository, build locally, run locally.
2. **Local build** — `./scripts/package_release.sh <version>` on the exact
   tagged commit reproduces the app bundle on your own machine.
3. **Hash-verifiable unsigned artifacts** — GitHub release assets carry an
   ad-hoc signature and a `SHA256SUMS` file derived from final bytes; trust is
   established by hash verification against the release manifest, never by
   Apple code-signing identity.

## Non-claims (permanent until explicitly changed)

- The app is **not** Developer ID signed. The embedded signature is ad-hoc
  (`codesign` reports `Signature=adhoc`, no `TeamIdentifier`), valid for
  on-disk integrity verification only.
- The app is **not** notarized.
- Gatekeeper **will refuse** double-click launch of a quarantined download.
  This is expected behavior for unsigned software, not a defect. Never treat
  an ad-hoc signature as a Developer ID signature.
- Optional future signing is a non-claim, never a blocker.

## Install from a GitHub release (hash-verified)

Download the three assets of a release into one directory:

- `NEMESIS-Desktop-v<version>-macos-arm64.zip`
- `release-manifest.json`
- `SHA256SUMS`

Verify bytes before anything else:

```sh
shasum -a 256 -c SHA256SUMS
python3 -c 'import json; m=json.load(open("release-manifest.json")); print(m["artifact"]["sha256"], m["source"]["commit"])'
```

Only after `shasum` reports `OK` for every asset:

```sh
ditto -x -k "NEMESIS-Desktop-v<version>-macos-arm64.zip" .
codesign --verify --deep --strict --verbose=2 "NEMESIS Desktop.app"
xattr -d com.apple.quarantine "NEMESIS Desktop.app" 2>/dev/null || true
open "NEMESIS Desktop.app"
```

Removing the quarantine attribute is safe **only because the bytes were hash
verified first**; the hash chain (release manifest → SHA256SUMS → your local
`shasum -c`) replaces the Gatekeeper trust decision.

## Install from source

```sh
git clone https://github.com/AnubisQuantumCipher/nemesis.git
cd nemesis
npm --prefix desktop ci
./scripts/build_ada.sh
cargo build --manifest-path runtime/Cargo.toml --workspace
./script/build_and_run.sh
```

Or reproduce the exact release bundle from the tagged commit:

```sh
git checkout v<version>
./scripts/package_release.sh <version>
```

## Upgrade

Replace the app bundle; user state is never inside it:

```sh
rm -rf "/Applications/NEMESIS Desktop.app"   # or wherever it was installed
ditto -x -k "NEMESIS-Desktop-v<new>-macos-arm64.zip" /Applications
```

Mission receipts, replay ledgers, and settings live in the local home
(`~/Library/Application Support/com.anubisquantumcipher.nemesis`) and survive
bundle replacement. The local home is schema-versioned
(`nemesis.local-home/v1`); an app that does not recognize the schema refuses
instead of guessing.

## Uninstall

```sh
rm -rf "NEMESIS Desktop.app"
```

Deliberate residuals (user data, removed only by explicit choice):

- `~/Library/Application Support/com.anubisquantumcipher.nemesis` — missions,
  ledgers, receipts, settings. Remove with `rm -rf` if desired.
- Keychain item service `dev.nemesis.receipt.seed.v1`, account
  `local-default` — the local receipt signing seed. Remove with:
  `security delete-generic-password -s dev.nemesis.receipt.seed.v1 -a local-default`.

The app installs no launchd jobs, no kernel extensions, no privileged
helpers, and never writes outside its bundle, its local home, and the
Keychain item above.

## Executable proof

`scripts/verify_install_contract.py` proves this contract end to end on real
bytes: hash verification, clean extract, ad-hoc signature validity, launch
(process + local-home creation under an isolated `$HOME`), upgrade with state
preservation across bundle replacement, version identity change, uninstall
with an explicit residual inventory. It writes
`receipts/production-readiness-20260821/INSTALL_CONTRACT.json` with a
`PASS_INSTALL_CONTRACT` terminal marker.
