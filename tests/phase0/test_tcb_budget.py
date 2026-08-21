from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.check_tcb_budget import count_ada_sloc


class TrustedComputingBaseCounterTests(unittest.TestCase):
    def test_counts_nonblank_noncomment_lines_only(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "sample.ads"
            source.write_text(
                "-- comment\n"
                "package Sample is\n"
                "\n"
                "   Value : constant := 1; -- inline comment\n"
                "end Sample;\n",
                encoding="utf-8",
            )
            self.assertEqual(3, count_ada_sloc([source]))

    def test_rejects_missing_source(self) -> None:
        with self.assertRaises(FileNotFoundError):
            count_ada_sloc([Path("does-not-exist.ads")])


if __name__ == "__main__":
    unittest.main()
