## Problem

Describe the observable gap or requirement. Link the governing issue/RFC when applicable.

## Decision

Describe the bounded change and why it fixes the source boundary rather than suppressing a symptom.

## Trust-boundary effect

- Authority/policy/capability/approval/budget effect:
- Evidence/receipt/completion effect:
- Sandbox/secret/network effect:
- Protocol or compatibility effect:

Write `none` only after checking the changed paths.

## Verification

List exact commands and observed terminal results. Do not report an unrun host-only gate as pass.

```text
command -> observed result
```

## Residuals and non-claims

Name deferred platforms, unavailable fixtures, untested environments, external assumptions, and claims this change does not establish.

## Checklist

- [ ] Behavioral or hostile regression failed before the fix and passes now, when behavior changed.
- [ ] Generated hashes/manifests were derived by their scripts.
- [ ] No credential, private build path, generated cache, or oversized artifact is staged.
- [ ] Security-sensitive changes use the private reporting path where appropriate.
- [ ] Commit sign-off follows `CONTRIBUTING.md`.
