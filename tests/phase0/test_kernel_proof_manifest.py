from __future__ import annotations

import unittest
from pathlib import Path

from scripts.verify_kernel_proof import (
    ProofValidationError,
    load_strict_json,
    parse_gnatprove_summary,
    validate_live_scope,
    validate_proof_summary,
)


ROOT = Path(__file__).resolve().parents[2]
SCOPE = ROOT / "config/formal-kernel-scope.json"

VALID_SUMMARY = """Summary of SPARK analysis
SPARK Analysis results        Total        Flow                      Provers   Justified   Unproved
Total                            78    36 (46%)                     42 (54%)           .          .

max steps used for successful proof: 2

Analyzed 2 units
in unit nemesis-kernel-approvals, 2 subprograms and packages out of 2 analyzed
  Nemesis.Kernel.Approvals at x flow analyzed (0 errors, 0 checks, 0 warnings and 0 pragma Assume statements) and proved (0 checks)
in unit nemesis-kernel-types, 5 subprograms and packages out of 5 analyzed
  Nemesis.Kernel.Types at x flow analyzed (0 errors, 0 checks, 0 warnings and 0 pragma Assume statements) and proved (0 checks)
"""


class KernelProofManifestTests(unittest.TestCase):
    def test_parses_nonzero_proved_summary_without_warnings_or_assumptions(self) -> None:
        summary = parse_gnatprove_summary(VALID_SUMMARY)

        self.assertEqual(summary.total_obligations, 78)
        self.assertEqual(summary.analyzed_units, 2)
        self.assertEqual(
            summary.units,
            {"nemesis-kernel-approvals", "nemesis-kernel-types"},
        )
        self.assertEqual(
            validate_proof_summary(
                summary,
                {"nemesis-kernel-approvals", "nemesis-kernel-types"},
            ),
            [],
        )

    def test_rejects_zero_work_missing_units_warnings_assumptions_and_timeouts(self) -> None:
        poisoned = VALID_SUMMARY.replace("Total                            78", "Total                             0")
        poisoned = poisoned.replace("Analyzed 2 units", "Analyzed 1 units")
        poisoned = poisoned.replace("0 warnings", "1 warnings", 1)
        poisoned = poisoned.replace("0 pragma Assume", "1 pragma Assume", 1)
        poisoned += "proof timed out\n"

        errors = validate_proof_summary(
            parse_gnatprove_summary(poisoned),
            {"nemesis-kernel-approvals", "nemesis-kernel-types"},
        )

        self.assertTrue(any("zero obligations" in error for error in errors))
        self.assertTrue(any("analyzed unit count" in error for error in errors))
        self.assertTrue(any("warnings" in error for error in errors))
        self.assertTrue(any("assumptions" in error for error in errors))
        self.assertTrue(any("timeout" in error for error in errors))

    def test_live_scope_covers_every_kernel_body_and_public_operation(self) -> None:
        scope = load_strict_json(SCOPE.read_text(encoding="utf-8"))

        self.assertEqual(validate_live_scope(scope, ROOT), [])
        self.assertEqual(len(scope["proved_units"]), 8)
        self.assertTrue(scope["unproved_authority_boundaries"])
        self.assertIn(
            "Derive_Child_Grant",
            scope["proved_units"]["nemesis-kernel-capabilities"]["public_operations"],
        )
        self.assertEqual(scope["trust_surface_blockers"], [])
        resolutions = scope["trust_surface_resolutions"]
        self.assertEqual(
            [entry["id"] for entry in resolutions], ["TS-001", "TS-002"]
        )
        for entry in resolutions:
            self.assertTrue(entry["resolution"])
            self.assertEqual(
                entry["authorized_by"]["contract_sha256"],
                "920900f3a7d37a9a3e1d51541997070789a0e1e7bddecf31808076c67a00d3f9",
            )

    def test_scope_refuses_silent_trust_surface_erasure(self) -> None:
        scope = load_strict_json(SCOPE.read_text(encoding="utf-8"))

        erased = dict(scope)
        erased["trust_surface_blockers"] = []
        erased["trust_surface_resolutions"] = []
        self.assertTrue(
            any(
                "trust surface state must remain explicit" in error
                for error in validate_live_scope(erased, ROOT)
            )
        )

        unanchored = dict(scope)
        unanchored["trust_surface_resolutions"] = [
            {
                "id": entry["id"],
                "description": entry["description"],
                "resolution": entry["resolution"],
                "authorized_by": {
                    "contract": entry["authorized_by"]["contract"],
                    "contract_sha256": "00" * 32,
                    "date": entry["authorized_by"]["date"],
                },
            }
            for entry in scope["trust_surface_resolutions"]
        ]
        self.assertTrue(
            any(
                "authorized_by" in error
                for error in validate_live_scope(unanchored, ROOT)
            )
        )

        duplicated = dict(scope)
        duplicated["trust_surface_blockers"] = [{"id": "TS-001"}]
        self.assertTrue(
            any(
                "ids are empty or duplicated" in error
                for error in validate_live_scope(duplicated, ROOT)
            )
        )

    def test_strict_scope_parser_rejects_duplicate_keys(self) -> None:
        with self.assertRaisesRegex(ProofValidationError, "duplicate JSON key"):
            load_strict_json('{"schema":"a","schema":"b"}')


if __name__ == "__main__":
    unittest.main()
