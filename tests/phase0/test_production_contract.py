from __future__ import annotations

import unittest
from pathlib import Path

from scripts.verify_production_contract import verify_contract


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "docs/mission/NEMESIS_DESKTOP_PRODUCTION_READINESS_AUTONOMOUS_MISSION_2026-08-21.md"


class ProductionContractTests(unittest.TestCase):
    def test_exact_contract_passes_and_one_byte_tamper_rejects(self) -> None:
        data = CONTRACT.read_bytes()

        self.assertEqual(verify_contract(data), [])
        tampered = bytearray(data)
        tampered[len(tampered) // 2] ^= 1
        self.assertTrue(verify_contract(bytes(tampered)))


if __name__ == "__main__":
    unittest.main()
