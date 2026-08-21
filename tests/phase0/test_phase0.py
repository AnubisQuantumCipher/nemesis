from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
REQUIRED = (
    ROOT / "rfcs/0001-constitution.md",
    ROOT / "THREAT_MODEL.md",
    ROOT / "TRUST_BOUNDARIES.md",
    ROOT / "SECURITY.md",
    ROOT / "GOVERNANCE.md",
    ROOT / "docs/verification/EVIDENCE_POLICY.md",
    ROOT / "docs/verification/DESKTOP_VERTICAL_SLICE_ACCEPTANCE.md",
)


class PhaseZeroContractTests(unittest.TestCase):
    def test_required_governance_documents_exist(self) -> None:
        missing = [str(path.relative_to(ROOT)) for path in REQUIRED if not path.is_file()]
        self.assertEqual([], missing, f"missing Phase 0 documents: {missing}")

    def test_constitution_locks_identity_and_authority(self) -> None:
        constitution = (ROOT / "rfcs/0001-constitution.md").read_text(encoding="utf-8")
        required = (
            "NEMESIS",
            "The Provable Agent Operating System",
            "Autonomy under control.",
            "Intelligence proposes. NEMESIS governs. Evidence decides.",
            "Workers may propose completion",
            "only the kernel may commit completion",
            "desktop-only",
            "trusted-computing-base budget",
        )
        for phrase in required:
            with self.subTest(phrase=phrase):
                self.assertIn(phrase, constitution)

    def test_evidence_policy_preserves_epistemic_statuses(self) -> None:
        policy = (ROOT / "docs/verification/EVIDENCE_POLICY.md").read_text(
            encoding="utf-8"
        )
        for word in ("VERIFIED", "BELIEVED", "UNKNOWN", "residual"):
            with self.subTest(word=word):
                self.assertIn(word, policy)

    def test_threat_model_covers_hostile_boundaries(self) -> None:
        model = (ROOT / "THREAT_MODEL.md").read_text(encoding="utf-8")
        for threat in (
            "prompt injection",
            "approval replay",
            "symlink",
            "secret exfiltration",
            "fabricated tool output",
            "daemon crash",
            "compromised frontend",
            "unavailable signer",
        ):
            with self.subTest(threat=threat):
                self.assertIn(threat, model.lower())

    def test_scope_marks_excluded_surfaces_deferred(self) -> None:
        boundaries = (ROOT / "TRUST_BOUNDARIES.md").read_text(
            encoding="utf-8"
        ).lower()
        for surface in (
            "iphone",
            "ipad",
            "web application",
            "public cloud",
            "windows",
            "linux",
            "notarization",
        ):
            with self.subTest(surface=surface):
                self.assertRegex(boundaries, rf"(?s){surface}.*deferred|deferred.*{surface}")
    def test_publication_is_bounded_to_the_authorized_alpha(self) -> None:
        boundaries = (ROOT / "TRUST_BOUNDARIES.md").read_text(encoding="utf-8").lower()
        for statement in (
            "public github source repository",
            "`v0.1.0` macos arm64 alpha",
            "not developer id signed or notarized",
            "package-registry publication",
            "legal-clearance review",
        ):
            with self.subTest(statement=statement):
                self.assertIn(statement, boundaries)


    def test_no_inherited_product_identifier_leaks_into_governance(self) -> None:
        offenders = []
        for path in REQUIRED:
            if path.is_file() and "SESHAT" in path.read_text(encoding="utf-8"):
                offenders.append(str(path.relative_to(ROOT)))
        self.assertEqual([], offenders)


if __name__ == "__main__":
    unittest.main()
