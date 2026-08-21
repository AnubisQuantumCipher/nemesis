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

- Commit: `efb73e92d12833e2912c28c437ee62002832f835`
- Tree: `541fe070768303ef326e395ecfafc6d4486f833a`
- Receipt: `receipts/release-v0.1.0/PUBLIC_ACCEPTANCE.json`
- Log: `receipts/release-v0.1.0/public-acceptance.txt`
- Log SHA-256: `32e53d079ba76232620b9013c92be4abfd3a84b22928a57db026d5a3f8f8f963`
- Log bytes: `83930`
- Log lines: `1562`
- Reconciled test-case executions: `200` (`21` Python, `171` Rust, `8` Vitest), including repeated host/VZ suites
- Tool identities: `receipts/release-v0.1.0/tool-versions.txt`

The test execution totals were reconciled against individual `ok` lines and Vitest file summaries before publication. Ada executable sentinels and GNATprove obligations are recorded separately in the same log; they are not folded into the test-case count.

The first public-clone attempt correctly stopped at `FAIL_EVIDENCE_BUNDLE` because a stale ignored duplicate artifact was not represented by the exact manifest. Its complete log and diagnosis are preserved in `PUBLIC_ACCEPTANCE.json`; the verifier was not weakened.

## Release-candidate hardening

- Subject commit: `443258079b996ad4c6bff209b93df51c59b23a3d`
- Subject tree: `b056224168006041affc029cd5cfef815f5628c1`
- Receipt: `receipts/phase-16-public/PHASE16.json`
- Complete gate log: `receipts/phase-16-public/security-hardening.txt`
- Gate log SHA-256: `63200915d61af19d8b1e1fab48b2d176d6acd6280fd52b450ef2a7ba66c1b004`
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
