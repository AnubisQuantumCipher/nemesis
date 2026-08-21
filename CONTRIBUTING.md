# Contributing to NEMESIS

NEMESIS accepts focused changes that preserve its authority and evidence boundaries. This is a one-maintainer alpha; review capacity and compatibility guarantees are limited.

## Before opening a change

1. Search existing issues and pull requests.
2. For security-sensitive behavior, use the private channel in [SECURITY.md](SECURITY.md), not a public issue.
3. For a protocol, trust-boundary, or public API change, open an RFC-oriented issue before implementation.
4. Keep mobile, web, remote-public, Windows/Linux packaging, signing/notarization, and package-registry work out of scope unless an accepted issue explicitly authorizes it.

## Development setup

Follow [README.md](README.md#prerequisites), then establish the local baseline:

```sh
npm --prefix desktop ci
./scripts/build_ada.sh
cargo build --manifest-path runtime/Cargo.toml --workspace
python3 scripts/verify_contract.py
python3 scripts/verify_release_contract.py
```

Work on a branch or isolated Git worktree. Do not commit generated `build/`, `target/`, `node_modules/`, `dist/`, `release/`, temporary receipts, local environment files, credentials, or Keychain material.

## Change discipline

- Add a failing behavioral or hostile regression before changing observable behavior.
- Fix the source boundary; do not delete an assertion, lower a threshold, special-case a failing fixture, or reinterpret a timeout/crash as pass.
- Keep worker/model output untrusted. Workers do not acquire authority, accept elevated evidence, or commit completion.
- Bind evidence to the final source revision. A later source change invalidates affected evidence until the relevant gate is rerun.
- Preserve historical receipts. New evidence epochs use new receipt files rather than rewriting published bytes.
- Do not introduce a second convention where an existing protocol, script, status vocabulary, or file layout already applies.

Changes to mission states, authority, policies, capabilities, approvals, budgets, evidence acceptance, receipt schemas, signing, secrets, sandboxing, or completion predicates require an RFC and the review described in [GOVERNANCE.md](GOVERNANCE.md). A change to what `PASS` means must be explicit and cannot land as routine cleanup.

## Verification

Run the narrow changed-surface tests first. Before requesting review, run the applicable repository gates:

```sh
python3 -m unittest discover -s tests -p 'test_*.py'
cargo fmt --manifest-path runtime/Cargo.toml --all -- --check
cargo test --manifest-path runtime/Cargo.toml --workspace
npm --prefix desktop test
npm --prefix desktop run build
cargo fmt --manifest-path desktop/src-tauri/Cargo.toml -- --check
cargo test --manifest-path desktop/src-tauri/Cargo.toml
```

For authority, storage, desktop integration, proof, or hardening changes, also run the corresponding scripts under `scripts/`. The full local roster is:

```sh
./scripts/verify_complete.sh
```

The VZ hardening sub-gate needs the private local Tart fixture described in the README. If you cannot run it, report it as **not run**; do not substitute a placeholder success.

## Commits and pull requests

- Stage paths explicitly; do not sweep unrelated files.
- Use a concise subject that states the change.
- Add `Signed-off-by:` to certify the [Developer Certificate of Origin](https://developercertificate.org/) intent:

  ```sh
  git commit -s -m "fix: describe the bounded change"
  ```

- In the pull request, explain the problem, decision, trust-boundary effect, exact commands run, observed results, and residual non-claims.
- Keep generated evidence and hash updates derived by their scripts. Do not type digest values by hand.

By contributing, you agree that your contribution is licensed under Apache-2.0.
