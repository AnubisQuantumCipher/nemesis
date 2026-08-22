# NEMESIS Desktop — Defensive Hardening Verification Brief

This is a **defensive code audit** for the application's own maintainers. The goal
is to confirm that the security invariants the project documents are actually
enforced in source, and to flag any place where a defensive check is missing or
weaker than its stated contract. Do not produce attack tooling; report findings as
maintainer-facing hardening guidance.

Subject is frozen. Audit exactly this state:

- Repository: `/Users/sicarii/Worktrees/nemesis-desktop-production-recovered`
- Subject commit: `HEAD` (the tip you are given; record its SHA)
- Inputs manifest: `receipts/production-readiness-20260821/security-review/INPUTS.json`

## Ground rules (mandatory)

1. READ-ONLY. Do not edit, build, install, spawn subagents, or run package managers. Static reading and `git log`/`git diff`/`git show` read commands only.
2. Defensive posture only. For each invariant, read the cited code and decide CONFIRMED (the check exists and is sufficient, cite `path:line`) or GAP (describe the missing/weaker validation in maintainer terms and the exact defensive fix). No exploit code, no attack walkthroughs.
3. You receive no lead reasoning. Base every statement on file bytes at the subject state; cite `path:line`.
4. Product scope: local macOS desktop application only — Tauri app in `desktop/`, Ada daemon in `daemon/`, SPARK kernel in `kernel/`, Rust runtime in `runtime/`, protocols in `protocols/`, gates in `scripts/` and `tests/`. Excluded: iOS/Android/web/Windows/Linux/SaaS.
5. Two known, already-documented and architect-gated hardening residuals — do not re-litigate, only note interactions: the daemon does not yet consume the kernel's one-shot approval inside its authorize path (TS-001); capability grants are synthesized rather than persisted parent + attenuated child (TS-002). Both are recorded in `config/formal-kernel-scope.json` and `docs/architecture/FORMAL_ASSURANCE.md`.
6. Context docs: `THREAT_MODEL.md`, `TRUST_BOUNDARIES.md`, `SECURITY.md`, `STATUS.md`, `docs/architecture/LOCAL_DESKTOP_MISSION.md`, `docs/architecture/FORMAL_ASSURANCE.md`, roster `receipts/production-readiness-20260821/PRODUCTION_READINESS.json`.

## Required output — one JSON object, nothing else

```json
{
  "schema": "nemesis.hardening-audit.result/v1",
  "lane": "<assigned lane id>",
  "reviewer_identity": {
    "context": "isolated subagent, separate process/context from lead",
    "model": "<your model identity as configured>",
    "limitations": ["static-only", "<other honest limits>"]
  },
  "subject_commit": "<the HEAD sha you read>",
  "coverage": [{"invariant": "<the documented property you checked>", "files": ["<paths>"], "status": "CONFIRMED|GAP"}],
  "gaps": [
    {
      "id": "<LANEPREFIX-NN>",
      "severity": "critical|high|medium|low|informational",
      "category": "<short category>",
      "cwe": "CWE-### or null",
      "path": "<file>",
      "line": <int or null>,
      "precondition": "<what state must exist for the gap to matter>",
      "impact": "<concrete maintainer-facing impact>",
      "evidence": "<static reasoning citing path:line>",
      "remediation": "<specific defensive fix>",
      "confidence": "high|medium|low"
    }
  ],
  "confirmed_invariants": ["<invariant + path:line where the defensive check is sufficient>"],
  "verdict": "CLEAN|GAPS|CRITICAL"
}
```

Verdict: `CLEAN` = every checked invariant CONFIRMED, no gap ≥ medium; `GAPS` = at least one medium/high gap; `CRITICAL` = a documented invariant is not enforced at all. Do not inflate severity; do not omit a real gap.
