# NEMESIS v0.1.0 Release Candidate Status

## Verdict

`PARTIAL` — the bounded desktop product and macOS release candidate have current local evidence, but required hosted GitHub checks, merge, tag, non-draft Release, and published-asset read-back have not yet occurred.

The sealed pre-release checkpoint remains at `receipts/CHECKPOINT.json`. Current local release evidence is `receipts/release-v0.1.0/LOCAL_ACCEPTANCE.json`.

## Local acceptance

Observed command:

```sh
./scripts/verify_complete.sh
```

Observed terminal marker:

```text
PASS_NEMESIS_DESKTOP_COMPLETE
```

Receipt subject:

- Commit: `c09ca87dc0c0f4c2961de915e00c3ba0b075b122`
- Tree: `4e4329f0f10d9865180078d5e030b3740a300bad`
- Log: `receipts/release-v0.1.0/local-acceptance.txt`
- Log SHA-256: `d04e0fea4970cf5a7c96219daccf78d6a84c4e20198a05a72ff1b2f04cb80f25`
- Log bytes: `83667`
- Log lines: `1560`
- Reconciled test-case executions: `198` (`19` Python, `171` Rust, `8` Vitest), including repeated host/VZ suites
- Tool identities: `receipts/release-v0.1.0/tool-versions.txt`

The test execution totals were reconciled against individual `ok` lines and Vitest file summaries before publication. Ada executable sentinels and GNATprove obligations are recorded separately in the same log; they are not folded into the test-case count.

## Release-candidate hardening

- Subject commit: `e74e5e15a8aaed027300c0df6cbdda104a0e6380`
- Subject tree: `abc7bfb8132fe573dcd6aef029e5c11fe604ae10`
- Receipt: `receipts/phase-16-release/PHASE16.json`
- Complete gate log: `receipts/phase-16-release/security-hardening.txt`
- Gate log SHA-256: `f0e8331f7e9deb11c2f6e41125e1e62f43c5463887d1d425cd25a6925b694d55`
- Observed: `PASS_PHASE16_SECURITY_HARDENING`
- Receipt verifier: `PASS_PHASE16_RECEIPT`
- Tamper regression: `PASS_PHASE16_RECEIPT_TAMPER_REJECTION`
- Cleanup: no `nemesis-desktop` process; only the stopped `anubis-xcode` Tart base remained

## Package rehearsal

```sh
./scripts/package_release.sh 0.1.0
```

Observed `PASS_RELEASE_PACKAGE` on the clean candidate source. The rehearsal app was macOS arm64, contained the standalone witnessed-mission backend and third-party notices, passed Mach-O RPATH/dependency sanitization, passed the seven-binary private-path/credential leak scan, passed strict ad-hoc code-signature verification, and produced a byte-derived manifest plus checksums. The release asset must be rebuilt from the final merged commit; rehearsal bytes are not publication bytes.

## Open hosted gates

1. Create/configure `AnubisQuantumCipher/nemesis` without unrelated remote history.
2. Push `main` and `build/desktop-local`, open the integration pull request, and require mission-owned checks to reach terminal success.
3. Merge without rewriting history; build assets from the exact merge commit.
4. Create and verify annotated tag `v0.1.0`.
5. Publish a non-draft prerelease and re-download every asset.
6. Commit a follow-up final evidence receipt containing hosted URLs, conclusions, sizes, and SHA-256 values.

## Residuals and non-claims

- Independent external security review remains `[NEEDS-HUMAN]`.
- SPARK evidence is limited to eight named kernel units and documented assumptions.
- The VZ result depends on the local macOS host, Tart base image, SSH transport, and toolchain.
- GNAT 14.2.1 deployment-target override warnings remain visible.
- Developer ID signing, notarization, Gatekeeper approval, App Store distribution, mobile, web, remote-public, Windows, Linux, package registration, domain acquisition, uniqueness, and legal clearance are not claimed.
