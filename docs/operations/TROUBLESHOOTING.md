# NEMESIS Desktop — Troubleshooting Matrix

Every command below runs against the installed build. Where a command targets an
installed app, substitute its resources path:
`RES="/Applications/NEMESIS Desktop.app/Contents/Resources"`.

| Symptom | Diagnostic command | Expected healthy result |
|---|---|---|
| App reports Core not ready | `python3 scripts/operator_diagnostics.py --resources "$RES"` | `core_daemon_health.status = OK`, `backend_inventory.nemesis_core_daemon.present = true`, marker `PASS_OPERATOR_DIAGNOSTICS` |
| Missions cannot start | inspect `backend_inventory` in diagnostics | every backend `present=true` and `executable=true`; `sandbox = WORKSPACE_SAFE_AVAILABLE` |
| "Local home not private" refusal | `ls -ld "$HOME/Library/Application Support/com.anubisquantumcipher.nemesis"` | mode `drwx------` (0700); not a symlink |
| Settings won't save | check free space and perms on the app home | writable `0700` dir, non-full volume; a full volume fails the write but preserves the prior `settings.json` (by design) |
| Suspected corrupt state after crash | relaunch the app | orphaned `.tmp-*` write temporaries are swept on init; a corrupt `home.json`/`settings.json`/`last-mission.json` is refused with a typed error, never half-loaded |
| Downgrade after upgrade | relaunch older build on newer data | refused: `unsupported local home schema` (downgrade guard) |
| Need to file a support request | `python3 scripts/support_bundle.py --home "$HOME/Library/Application Support/com.anubisquantumcipher.nemesis" --out /tmp/nemesis-support` | `SUPPORT_BUNDLE_READY ... transmission=disabled`; review `/tmp/nemesis-support/nemesis-support/MANIFEST.json` before sharing |
| Verify a receipt/replay | `"$RES/bin/nemesis-verify" ...` / `"$RES/bin/nemesis-replay" --ledger <events.ledger>` | verifier present; replay prints `"verdict":"VERIFIED"` for an intact ledger |
| Confirm no network egress | `python3 scripts/operator_diagnostics.py` | `network = DENIED_BY_CONTRACT`, `updates = DISABLED_NO_AUTHENTICATED_UPDATER` |
| Reset the application entirely | remove the app home directory | next launch is a clean first-run install with defaulted settings (all user data deleted) |

## Escalation

If diagnostics show all backends present and Core health `OK` but the UI still
refuses a mission, the refusal is intentional and typed — read the on-screen
`code`, `message`, and `recovery` fields; they name the exact seam (workspace
identity mismatch, digest mismatch, denied authority, or bounded-resource limit).
Attach the support bundle (after review) when reporting per `SECURITY.md`.
