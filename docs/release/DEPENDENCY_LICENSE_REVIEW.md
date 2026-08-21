# Dependency License Review

**Observed:** 2026-08-20  
**Release:** NEMESIS Desktop `v0.1.0` macOS arm64 alpha

## Policy and commands

Rust dependency expressions are evaluated by pinned `cargo-about 0.9.2` against [`runtime/about.toml`](../../runtime/about.toml):

```sh
cargo about generate \
  --manifest-path runtime/Cargo.toml \
  --workspace \
  --config runtime/about.toml \
  --format json \
  --output-file /tmp/runtime-licenses.json \
  --locked \
  --fail

cargo about generate \
  --manifest-path desktop/src-tauri/Cargo.toml \
  --config runtime/about.toml \
  --format json \
  --output-file /tmp/desktop-licenses.json \
  --locked \
  --fail
```

Both commands exited zero on the reviewed lockfiles. The accepted set is limited to permissive licenses plus MPL-2.0's file-level terms. No dependency is accepted merely because the resolver can parse its expression; an unaccepted or unidentified expression makes `--fail` nonzero.

Production npm dependencies are derived from `desktop/package-lock.json` entries that are not marked development-only. After `npm ci`, every shipped npm package must contain a non-empty `LICENSE`, `COPYING`, or `NOTICE`-prefixed file.

## Distribution output

`scripts/generate_third_party_notices.py` combines:

- crate name, version, declared expression, package-specific license/notice text, and cargo-about attribution;
- production npm package name, version, declared expression, and packaged license files.

`scripts/package_release.sh` generates `build/release/THIRD_PARTY_NOTICES.txt` before the Tauri bundle step and embeds it in `NEMESIS Desktop.app/Contents/Resources/`. Generation fails on missing/empty license text. The release also embeds NEMESIS's Apache-2.0 `LICENSE`.

The notice inventory names exact crate/package versions. Corresponding unmodified dependency source can be obtained from the crates.io/npm registry identities pinned by the committed lockfiles.

## Boundary

This is an engineering inventory and distribution control, not legal advice. It does not establish trademark clearance, patent non-infringement, or legal compatibility for a use outside the shipped source-visible alpha. Dependency licenses can change when lockfiles change; the release packager and hosted dependency job regenerate and re-evaluate the inventory rather than carrying this observation forward unchanged.
