# NEMESIS Desktop — Privacy, Data, and Operations

This document matches the shipped behavior of the local macOS desktop application.
Every claim below maps to source or a runnable command; nothing here describes a
capability that is not implemented.

## Telemetry and network posture

- **No telemetry.** The application performs no analytics, crash-reporting, or
  usage collection. There is no opt-in telemetry channel and no background
  reporter process.
- **Network denied by default.** Mission authority is fail-closed: contracts set
  `authority.network=false` and the runtime reports `network=DENIED_BY_CONTRACT`
  (`desktop/src-tauri/src/lib.rs` `system_status`). No prompts, repository
  content, secrets, or mission evidence leave the Mac without an explicit bounded
  authority grant, which this product does not enable for network egress.

## Where your data lives

All state is local and private (directories `0700`, files `0600`):

- Application home: `~/Library/Application Support/com.anubisquantumcipher.nemesis`
  - `home.json` — versioned local-home manifest
  - `settings.json` — text-scale and reduced-motion preferences
  - `drafts/`, `missions/`, `lanes/`, `receipts/`, `logs/`, `support/`, `tmp/`
  - `.core/<mission-prefix>/` — per-mission Core control home and event ledger
- No data is written outside this directory except the workspace you explicitly
  point a mission at.

## Backup and restore

- **Backup:** copy the application home directory above while the app is closed.
- **Restore:** copy it back to the same path. The versioned manifest refuses a
  newer on-disk schema than the installed binary supports (downgrade guard).

## Export and delete

- **Export:** the application home is plain files; copy it, or generate a bounded
  support bundle (below).
- **Delete all user data (uninstall):** remove
  `~/Library/Application Support/com.anubisquantumcipher.nemesis`. The next launch
  is a clean first-run install with defaulted settings.

## Log redaction

- Release binaries are built with `--remap-path-prefix` so absolute build and
  home paths do not appear in shipped bytes (`scripts/package_release.sh`).
- Logs are bounded and contain no secrets. The support-bundle generator
  additionally scans every collected byte and refuses to emit a bundle if a
  secret-like pattern is found.

## Operator diagnostics

Run, against the in-tree build or an installed app's `Resources`:

```
python3 scripts/operator_diagnostics.py            # in-tree
python3 scripts/operator_diagnostics.py --resources "/Applications/NEMESIS Desktop.app/Contents/Resources"
```

Reports Core/Kernel/Runtime identity, backend inventory, Core daemon health,
receipt/replay verifier presence, sandbox availability, and the network/update
posture. Emits no secrets. Marker: `PASS_OPERATOR_DIAGNOSTICS`.

## Support bundle (bounded, reviewable, never auto-sent)

```
python3 scripts/support_bundle.py --home "~/Library/Application Support/com.anubisquantumcipher.nemesis" --out /tmp/nemesis-support
```

The bundle is size-capped, secret-scanned (refuses on any match), lists every
inclusion in `MANIFEST.json` for your review, and is **never transmitted
automatically** (`transmission=DISABLED_NO_AUTOMATIC_EXPORT`). You decide whether
to share it.

## Updates

- Automatic updates are **explicitly excluded**; no updater is shipped
  (`updates=DISABLED_NO_AUTHENTICATED_UPDATER`). The product never ships an
  unauthenticated updater. Obtain new versions only from the authoritative signed
  release artifact.

## Reporting a vulnerability

See `SECURITY.md` for the coordinated disclosure contact and policy.
