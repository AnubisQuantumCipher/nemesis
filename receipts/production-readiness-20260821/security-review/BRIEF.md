# NEMESIS Desktop — Independent Autonomous External Review Brief

Subject is frozen. Review exactly this state:

- Repository: `/Users/sicarii/Worktrees/nemesis-desktop-production-recovered`
- Subject commit: `f36d84b267af73bc559b89a76ce6c9a6c57bb770`
- Subject tree: `88f83b2e4aa52176bc104e1fa1596b9fa04e5cd4`
- Inputs manifest: `receipts/production-readiness-20260821/security-review/INPUTS.json`

## Ground rules (mandatory)

1. READ-ONLY. Do not edit, build, install, spawn subagents, or run package managers. Static analysis and file reads only. `git log`/`git diff` read commands are allowed.
2. Defensive review only. No exploit development beyond precise precondition/impact reasoning.
3. You receive no lead reasoning. Base every claim on file bytes at the subject state; cite `path:line`.
4. Product scope: local macOS desktop application only (Tauri app in `desktop/`, Ada daemon in `daemon/`, SPARK kernel in `kernel/`, Rust runtime in `runtime/`, protocols in `protocols/`, gates in `scripts/` and `tests/`). Excluded: iOS/Android/web/Windows/Linux/SaaS.
5. Known accepted trust-surface blockers (do not re-litigate; note interactions only): TS-001 (daemon does not consume one-shot kernel approvals in `authorize_action`), TS-002 (synthesized capability grant instead of persisted parent + attenuated child). Both are documented in `config/formal-kernel-scope.json` and `docs/architecture/FORMAL_ASSURANCE.md` and are architect-gated.
6. Context docs: `THREAT_MODEL.md`, `TRUST_BOUNDARIES.md`, `SECURITY.md`, `STATUS.md`, `docs/architecture/LOCAL_DESKTOP_MISSION.md`, `docs/architecture/FORMAL_ASSURANCE.md`, roster `receipts/production-readiness-20260821/PRODUCTION_READINESS.json`.

## Required output — one JSON object, nothing else

```json
{
  "schema": "nemesis.security-review.finding-set/v1",
  "lane": "<assigned lane id>",
  "reviewer_identity": {
    "context": "isolated OMP subagent, separate process/context from lead",
    "model": "<your model identity as configured>",
    "limitations": ["<honest limitations, e.g. static-only, no dynamic execution>"]
  },
  "subject": {"commit": "f36d84b267af73bc559b89a76ce6c9a6c57bb770", "tree": "88f83b2e4aa52176bc104e1fa1596b9fa04e5cd4"},
  "coverage": [{"area": "<what you examined>", "files": ["<paths>"]}],
  "findings": [
    {
      "id": "<LANEPREFIX-NN>",
      "severity": "critical|high|medium|low|informational",
      "category": "<short category>",
      "cwe": "CWE-### or null",
      "path": "<file>",
      "line": <int or null>,
      "exploit_precondition": "<exact precondition>",
      "impact": "<concrete impact>",
      "reproduction": "<static reasoning or exact repro steps>",
      "remediation": "<specific fix>",
      "confidence": "high|medium|low"
    }
  ],
  "non_findings": ["<important negative results you verified, with path:line>"],
  "verdict": "PASS|FINDINGS|FAIL"
}
```

Verdict semantics: `PASS` = no findings at severity ≥ medium; `FINDINGS` = at least one medium/high; `FAIL` = at least one critical or a broken trust boundary. Do not inflate severity; do not omit real findings.
