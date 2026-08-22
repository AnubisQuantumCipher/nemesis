#!/usr/bin/env bash
# Phase L — hostile and tamper control calibration.
# Drives the real production verifiers with synthetic poison on disposable copies
# and asserts each control REFUSES. No real artifact is mutated in place; the
# A->B->A probe restores exact bytes. Captures status/timestamp/tool/hash/marker.
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

VALIDATOR="scripts/validate_production_readiness.py"
ROSTER="receipts/boss-20260822/PRODUCTION_READINESS.json"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
fails=0

expect_refuse() { # label, cmd...
  local label="$1"; shift
  if "$@" >/dev/null 2>&1; then
    echo "FAIL_HOSTILE $label DID_NOT_REFUSE"; fails=$((fails + 1))
  else
    echo "OK_REFUSED $label"
  fi
}
expect_accept() { # label, cmd...
  local label="$1"; shift
  if "$@" >/dev/null 2>&1; then
    echo "OK_ACCEPTED $label"
  else
    echo "FAIL_HOSTILE $label REFUSED_VALID"; fails=$((fails + 1))
  fi
}

# Instrument sanity: the pristine roster validates.
expect_accept "baseline-roster-valid" python3 "$VALIDATOR" "$ROSTER"

# L-01 zero-work / empty corpus: a roster with no lanes must fail (required lanes absent).
python3 - "$ROSTER" "$WORK/empty.json" <<'PY'
import json, sys
data = json.load(open(sys.argv[1])); data["lanes"] = []
json.dump(data, open(sys.argv[2], "w"))
PY
expect_refuse "L-01-empty-corpus" python3 "$VALIDATOR" "$WORK/empty.json"

# L-02 malformed / duplicate key: strict JSON must reject duplicate object keys.
head -c -2 "$ROSTER" >/dev/null 2>&1 || true
python3 - "$ROSTER" "$WORK/dup.json" <<'PY'
import sys
text = open(sys.argv[1]).read().rstrip()
# Inject a duplicate top-level key by reopening the object.
assert text.endswith("}")
open(sys.argv[2], "w").write(text[:-1] + ',"overall":"PASS"}')
PY
expect_refuse "L-02-duplicate-key" python3 "$VALIDATOR" "$WORK/dup.json"

# L-02 contradictory: a SKIPPED/BLOCKED lane may not coexist with overall PASS.
python3 - "$ROSTER" "$WORK/contra.json" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
data["overall"] = "PASS"
data["lanes"][0]["status"] = "SKIPPED"
json.dump(data, open(sys.argv[2], "w"))
PY
expect_refuse "L-02-contradictory-overall" python3 "$VALIDATOR" "$WORK/contra.json"

# L-08 unsafe artifact path: absolute / parent-dir evidence paths must be refused.
python3 - "$ROSTER" "$WORK/unsafe.json" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
data["lanes"][0]["evidence"] = [{
    "id": "poison-1",
    "classification": "CANONICAL",
    "command": "probe",
    "exit_status": 0,
    "subject": data.get("subject"),
    "artifact": {"path": "../../etc/passwd", "sha256": "ab" * 32},
    "terminal_marker": "PASS_PROBE",
}]
json.dump(data, open(sys.argv[2], "w"))
PY
expect_refuse "L-08-unsafe-artifact-path" python3 "$VALIDATOR" "$WORK/unsafe.json"

# L-06 alternate optimized interpreter: -O strips asserts; load-bearing checks must
# NOT depend on assert. The duplicate-key poison must still be refused under -O.
expect_refuse "L-06-python-O-strips-assert" python3 -O "$VALIDATOR" "$WORK/dup.json"

# L-05 stale PASS cleanup: a stale PASS marker file must not make a poison roster pass.
echo "PASS_PRODUCTION_ROSTER_VALID stale" >"$WORK/stale.marker"
expect_refuse "L-05-stale-pass-ignored" python3 "$VALIDATOR" "$WORK/contra.json"

# L-09 exact-byte A->B->A on a disposable copy.
cp "$ROSTER" "$WORK/aba.json"
A_HASH="$(shasum -a 256 "$WORK/aba.json" | cut -d' ' -f1)"
python3 "$VALIDATOR" "$WORK/aba.json" >/dev/null 2>&1 && A_OK=1 || A_OK=0
printf '\x00' >>"$WORK/aba.json"   # B: tamper
python3 "$VALIDATOR" "$WORK/aba.json" >/dev/null 2>&1 && B_OK=1 || B_OK=0
cp "$ROSTER" "$WORK/aba.json"       # restore exact bytes
A2_HASH="$(shasum -a 256 "$WORK/aba.json" | cut -d' ' -f1)"
python3 "$VALIDATOR" "$WORK/aba.json" >/dev/null 2>&1 && A2_OK=1 || A2_OK=0
if [[ "$A_OK" == 1 && "$B_OK" == 0 && "$A2_OK" == 1 && "$A_HASH" == "$A2_HASH" ]]; then
  echo "OK_REFUSED L-09-aba-exact-restore A=$A_OK B=$B_OK A2=$A2_OK bytes_restored=true"
else
  echo "FAIL_HOSTILE L-09-aba A=$A_OK B=$B_OK A2=$A2_OK hash_match=$([[ $A_HASH == $A2_HASH ]] && echo true || echo false)"
  fails=$((fails + 1))
fi

# L-07 changed tracked/untracked inventory detection.
probe="receipts/production-readiness-20260821/.hostile-inventory-probe"
: >"$probe"
if git status --porcelain --untracked-files=all -- "$probe" | grep -q '.'; then
  echo "OK_REFUSED L-07-inventory-change-detected"
else
  echo "FAIL_HOSTILE L-07-inventory-change-not-detected"; fails=$((fails + 1))
fi
rm -f "$probe"

# L-03/L-04 covered-source + unlisted-artifact controls are enforced by the phase16
# verifier (source/tree binding) and the evidence-bundle verifier (unlisted reject),
# exercised in tests.integration and by verify_complete; referenced here, not duplicated.
echo "NOTE L-03/L-04 enforced by scripts/verify_phase16_receipt.py (source/tree binding) and scripts/verify_evidence_bundle.py (unlisted/missing/stale reject)"

if [[ "$fails" -ne 0 ]]; then
  echo "FAIL_HOSTILE_CALIBRATION fails=$fails"
  exit 1
fi
printf '%s\n' 'PASS_HOSTILE_CALIBRATION'
