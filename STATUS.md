# NEMESIS v0.1.0 Status

## Verdict

`COMPLETE` — `./scripts/verify_complete.sh` at commit `4e10cfcfeb3adf20ced7a01309ac0b2fb028373b` ended `PASS_NEMESIS_DESKTOP_COMPLETE`; [final-merge CI run 32447296563](https://github.com/AnubisQuantumCipher/nemesis/actions/runs/32447296563) concluded `success`; the [v0.1.0 GitHub Release](https://github.com/AnubisQuantumCipher/nemesis/releases/tag/v0.1.0) is non-draft; fresh downloads matched all local asset bytes and SHA-256 values.

Authoritative machine receipt: [`receipts/release-v0.1.0/FINAL.json`](receipts/release-v0.1.0/FINAL.json).

## Release identity

- Repository: https://github.com/AnubisQuantumCipher/nemesis
- Integration PR: https://github.com/AnubisQuantumCipher/nemesis/pull/1
- Packaging-fix PR: https://github.com/AnubisQuantumCipher/nemesis/pull/2
- Release commit: `4e10cfcfeb3adf20ced7a01309ac0b2fb028373b`
- Release tree: `e6a3a6f2aeac1eea53dc2ec089f6156e174c5746`
- Annotated tag: `v0.1.0` → `4e10cfcfeb3adf20ced7a01309ac0b2fb028373b`
- Release: https://github.com/AnubisQuantumCipher/nemesis/releases/tag/v0.1.0
- Published state: non-draft prerelease

## Final local acceptance

```text
./scripts/verify_complete.sh -> PASS_NEMESIS_DESKTOP_COMPLETE
```

- Subject: release commit/tree above
- Log: `receipts/release-v0.1.0/tagged-tree-acceptance.txt`
- SHA-256: `c4dcfb60d2f8b6294d7d639517f7877a1c4a38d58e309dc3ac02a38bb58bcecc`
- Bytes: `99377`
- Lines: `1992`
- Reconciled test-case executions: `200` (`21` Python, `171` Rust, `8` Vitest), including repeated host/VZ suites
- Phase 16 receipt: `receipts/phase-16-public/PHASE16.json`
- Cleanup: no `nemesis-desktop` process; only stopped `anubis-xcode` remained

Ada executable sentinels and GNATprove obligations are recorded separately in the same log and are not folded into the test-case count.

## Hosted checks

All jobs ran against final merge SHA `4e10cfcfeb3adf20ced7a01309ac0b2fb028373b`:

- [Contract and evidence](https://github.com/AnubisQuantumCipher/nemesis/actions/runs/32447296563/job/96669144974) — `success`
- [Rust and Desktop (macos-14)](https://github.com/AnubisQuantumCipher/nemesis/actions/runs/32447296563/job/96669145165) — `success`
- [Rust and Desktop (macos-15)](https://github.com/AnubisQuantumCipher/nemesis/actions/runs/32447296563/job/96669145210) — `success`
- [Dependency audit](https://github.com/AnubisQuantumCipher/nemesis/actions/runs/32447296563/job/96669145215) — `success`

## Package and published assets

```text
./scripts/package_release.sh 0.1.0 -> PASS_RELEASE_PACKAGE
```

Final package log: `receipts/release-v0.1.0/package-final.txt` (`fa4e08c3befa4b2e5f6b009a77149b02d19b6e858bce3dae3994676578ad5516` SHA-256).

The first exact-merge package attempt correctly failed when `codesign` rejected Finder/resource-fork metadata. Its log is preserved at `receipts/release-v0.1.0/package-attempt-1.txt`; PR #2 added generated-bundle metadata cleanup without changing any verifier or signing condition.

| Asset | Bytes | SHA-256 |
|---|---:|---|
| `NEMESIS-Desktop-v0.1.0-macos-arm64.zip` | 4,330,327 | `6496b646043face3c0e907dcc7a01ca4a2b049a637b3b62743545bc674f22753` |
| `release-manifest.json` | 1,067 | `dda0e7ea4261a8400f1d986ffbb62a1ef0a25907363471c983508596ade3c08e` |
| `SHA256SUMS` | 193 | `cfa14179a19ed33fbc4504af1a72b4e709431d0eb0ccfc4c515c7e92ba487db1` |

Fresh `gh release download` bytes passed `shasum -a 256 -c SHA256SUMS`, independent SHA-256 checks, and `cmp` against the final local assets.

The extracted app passed strict ad-hoc `codesign` verification, launched a 1440×900 window titled “NEMESIS — The Provable Agent Operating System,” ran the bundled witnessed mission, accepted the valid receipt, and rejected the tampered receipt. `spctl -a -vv` rejected the app as expected for non-notarized ad-hoc distribution.

## Repository controls

- Public repository, Apache-2.0 license metadata, issues, topics, and private vulnerability reporting are active.
- `main` requires pull requests, strict success from the four checks above, and conversation resolution.
- Force pushes and branch deletion are disabled.
- No homepage is configured because no project site exists.

## Residuals and non-claims

- Independent external security review remains `[NEEDS-HUMAN]`.
- SPARK evidence is limited to eight named kernel units and documented assumptions.
- The VZ result depends on the local macOS host, Tart base image, SSH transport, and toolchain.
- GNAT 14.2.1 deployment-target override warnings remain visible.
- Developer ID signing, notarization, Gatekeeper approval, App Store distribution, mobile, web, remote-public, Windows, Linux, package registration, domain acquisition, uniqueness, and legal clearance are not claimed.

The published tag remains immutably bound to the release commit. This status and `FINAL.json` are a post-tag evidence follow-up; they do not move or rewrite `v0.1.0`.
