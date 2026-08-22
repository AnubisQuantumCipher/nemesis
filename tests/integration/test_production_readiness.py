from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path

from scripts.validate_production_readiness import (
    REQUIRED_LANES,
    ValidationError,
    parse_strict_json,
    validate_roster,
)


SUBJECT = {
    "commit": "1" * 40,
    "tree": "2" * 40,
}


def fixture(root: Path) -> dict[str, object]:
    artifact = root / "gate.log"
    artifact.write_text("PASS_GATE\n", encoding="utf-8")
    digest = hashlib.sha256(artifact.read_bytes()).hexdigest()
    evidence = {
        "id": "gate",
        "classification": "FOCUSED",
        "command": "./gate",
        "exit_status": 0,
        "terminal_marker": "PASS_GATE",
        "artifact": {"path": "gate.log", "sha256": digest},
        "subject": SUBJECT,
    }
    return {
        "schema": "nemesis.production-readiness/v1",
        "subject": SUBJECT,
        "overall": "PASS",
        "automatable_complete": True,
        "terminal_verdict": "COMPLETE_PRODUCTION_PUBLIC",
        "external_blockers": [],
        "lanes": [
            {
                "id": lane_id,
                "required": True,
                "status": "PASS",
                "evidence": [deepcopy(evidence)],
                "blockers": [],
            }
            for lane_id in sorted(REQUIRED_LANES)
        ],
    }


class ProductionReadinessValidatorTests(unittest.TestCase):
    def test_accepts_complete_roster_with_all_required_lanes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.assertEqual(validate_roster(fixture(root), root), [])

    def test_rejects_duplicate_json_keys_before_values_are_overwritten(self) -> None:
        with self.assertRaisesRegex(ValidationError, "duplicate JSON key: schema"):
            parse_strict_json('{"schema":"first","schema":"second"}')

    def test_rejects_duplicate_lane_ids_and_missing_required_lane(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            roster = fixture(root)
            roster["lanes"][1]["id"] = roster["lanes"][0]["id"]

            errors = validate_roster(roster, root)

        self.assertTrue(any("duplicate lane id" in error for error in errors))
        self.assertTrue(any("required lane set" in error for error in errors))

    def test_rejects_overall_pass_when_any_lane_is_not_pass(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            roster = fixture(root)
            roster["lanes"][0]["status"] = "UNKNOWN"
            roster["lanes"][0]["evidence"] = []

            errors = validate_roster(roster, root)

        self.assertTrue(any("overall contradicts derived state" in error for error in errors))
        self.assertTrue(any("COMPLETE_PRODUCTION_PUBLIC" in error for error in errors))

    def test_rejects_pass_lane_without_current_source_bound_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            roster = fixture(root)
            roster["lanes"][0]["evidence"] = []

            errors = validate_roster(roster, root)

        self.assertTrue(any("PASS requires evidence" in error for error in errors))

    def test_rejects_evidence_bound_to_another_subject_or_missing_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            roster = fixture(root)
            evidence = roster["lanes"][0]["evidence"][0]
            evidence["subject"] = {"commit": "3" * 40, "tree": "4" * 40}
            evidence["artifact"]["path"] = "missing.log"

            errors = validate_roster(roster, root)

        self.assertTrue(any("evidence subject mismatch" in error for error in errors))
        self.assertTrue(any("artifact unreadable" in error for error in errors))

    def test_rejects_symlink_substitution_for_evidence_artifact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            roster = fixture(root)
            (root / "linked.log").symlink_to(root / "gate.log")
            roster["lanes"][0]["evidence"][0]["artifact"]["path"] = "linked.log"

            errors = validate_roster(roster, root)

        self.assertTrue(any("artifact must not be a symlink" in error for error in errors))

    def test_rejects_public_blocker_verdict_until_automatable_lanes_are_done(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            roster = fixture(root)
            roster["terminal_verdict"] = "BLOCKED_PRODUCTION_PUBLIC"
            roster["automatable_complete"] = False
            roster["external_blockers"] = [
                {
                    "id": "developer-id",
                    "kind": "EXTERNAL",
                    "command": "codesign probe",
                    "exit_status": 1,
                }
            ]
            roster["lanes"][0]["status"] = "BLOCKED"
            roster["lanes"][0]["evidence"] = []
            roster["lanes"][0]["blockers"] = roster["external_blockers"]
            roster["overall"] = "BLOCKED"

            errors = validate_roster(roster, root)

        self.assertTrue(any("automatable_complete" in error for error in errors))

    def test_sealed_local_production_requires_pass_lanes_and_nonclaims(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sealed = fixture(root)
            sealed["terminal_verdict"] = "SEALED_LOCAL_PRODUCTION"
            sealed["nonclaims"] = [
                "The app is not Developer ID signed.",
                "The app is not notarized.",
                "Gatekeeper acceptance is not claimed.",
            ]
            sealed["external_blockers"] = [
                {
                    "id": "external-unlocked-console",
                    "kind": "EXTERNAL",
                    "required_for_verdict": False,
                    "finding": "G-10 native walkthrough deferred: IOConsoleLocked=Yes",
                }
            ]
            self.assertEqual(validate_roster(sealed, root), [])

            missing_nonclaims = deepcopy(sealed)
            missing_nonclaims["nonclaims"] = ["unsigned"]
            errors = validate_roster(missing_nonclaims, root)
            self.assertTrue(any("nonclaims" in error for error in errors))

            blocked_lane = deepcopy(sealed)
            blocked_lane["lanes"][0]["status"] = "BLOCKED"  # type: ignore[index]
            blocked_lane["overall"] = "BLOCKED"
            errors = validate_roster(blocked_lane, root)
            self.assertTrue(
                any("SEALED_LOCAL_PRODUCTION requires every lane PASS" in error
                    for error in errors)
            )

            load_bearing_blocker = deepcopy(sealed)
            load_bearing_blocker["external_blockers"] = [
                {"id": "x", "kind": "EXTERNAL", "required_for_verdict": True}
            ]
            errors = validate_roster(load_bearing_blocker, root)
            self.assertTrue(
                any("informational EXTERNAL blockers" in error for error in errors)
            )

    def test_strict_parser_round_trips_a_valid_roster(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            roster = fixture(root)
            parsed = parse_strict_json(json.dumps(roster))
            self.assertEqual(validate_roster(parsed, root), [])


if __name__ == "__main__":
    unittest.main()
