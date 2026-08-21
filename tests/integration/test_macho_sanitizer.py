from __future__ import annotations

import unittest

from scripts.sanitize_macho import private_rpaths


class MachOSanitizerTests(unittest.TestCase):
    def test_private_absolute_rpaths_are_selected_for_removal(self) -> None:
        output = """Load command 1
          cmd LC_RPATH
      cmdsize 128
         path /Users/builder/toolchain/lib (offset 12)
Load command 2
          cmd LC_RPATH
      cmdsize 32
         path @loader_path (offset 12)
Load command 3
          cmd LC_RPATH
      cmdsize 64
         path /private/tmp/build/lib (offset 12)
"""

        self.assertEqual(
            private_rpaths(output),
            ["/Users/builder/toolchain/lib", "/private/tmp/build/lib"],
        )


if __name__ == "__main__":
    unittest.main()
