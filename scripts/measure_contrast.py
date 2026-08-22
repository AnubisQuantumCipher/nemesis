#!/usr/bin/env python3
"""Phase G-03 contrast instrument.

Parses the desktop design tokens from desktop/src/styles.css and computes WCAG 2.1
contrast ratios for every load-bearing semantic foreground/background pair. Fails
closed if a required pair is below its threshold. Normal text requires >= 4.5:1;
large text and non-text UI (borders, focus rings, glyph indicators) require >= 3.0:1.

This validates the instrument before publishing numbers: the reference pair
white-on-black must compute to 21.00 or the parser is rejected.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CSS = ROOT / "desktop/src/styles.css"


def parse_tokens(text: str) -> dict[str, str]:
    tokens: dict[str, str] = {}
    for name, value in re.findall(r"--([a-z0-9-]+):\s*(#[0-9a-fA-F]{6})\s*;", text):
        tokens[name] = value
    return tokens


def srgb_to_linear(channel: float) -> float:
    return channel / 12.92 if channel <= 0.03928 else ((channel + 0.055) / 1.055) ** 2.4


def relative_luminance(hex_color: str) -> float:
    hex_color = hex_color.lstrip("#")
    r, g, b = (int(hex_color[i : i + 2], 16) / 255.0 for i in (0, 2, 4))
    r, g, b = (srgb_to_linear(c) for c in (r, g, b))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contrast(fg: str, bg: str) -> float:
    l1, l2 = relative_luminance(fg), relative_luminance(bg)
    lighter, darker = max(l1, l2), min(l1, l2)
    return (lighter + 0.05) / (darker + 0.05)


# (label, foreground token, background token, threshold, kind)
PAIRS = [
    ("primary text on black", "text", "black", 4.5, "text"),
    ("primary text on graphite-1", "text", "graphite-1", 4.5, "text"),
    ("primary text on graphite-2", "text", "graphite-2", 4.5, "text"),
    ("primary text on graphite-3", "text", "graphite-3", 4.5, "text"),
    ("secondary text on black", "text-secondary", "black", 4.5, "text"),
    ("secondary text on graphite-1", "text-secondary", "graphite-1", 4.5, "text"),
    ("muted text on black", "text-muted", "black", 4.5, "text"),
    ("muted text on graphite-1", "text-muted", "graphite-1", 4.5, "text"),
    ("muted text on graphite-2", "text-muted", "graphite-2", 4.5, "text"),
    ("authority red on black", "red", "black", 3.0, "ui"),
    ("authority red-bright on graphite-1", "red-bright", "graphite-1", 4.5, "text"),
    ("verified green on black", "green", "black", 3.0, "ui"),
    ("verified green on graphite-1", "green", "graphite-1", 3.0, "ui"),
    ("amber on black", "amber", "black", 3.0, "ui"),
    ("focus ring on black", "focus", "black", 3.0, "ui"),
    ("focus ring on graphite-2", "focus", "graphite-2", 3.0, "ui"),
    ("strong line on black", "line-strong", "black", 3.0, "ui"),
]


def main() -> int:
    tokens = parse_tokens(CSS.read_text(encoding="utf-8"))
    # Instrument self-validation: white on black is exactly 21.00:1.
    reference = contrast("#ffffff", "#000000")
    if round(reference, 2) != 21.00:
        print(f"FAIL_CONTRAST_INSTRUMENT reference={reference:.2f} expected=21.00")
        return 2

    results = []
    failures = []
    for label, fg, bg, threshold, kind in PAIRS:
        if fg not in tokens or bg not in tokens:
            print(f"FAIL_CONTRAST missing_token label={label!r} fg={fg} bg={bg}")
            return 2
        ratio = contrast(tokens[fg], tokens[bg])
        ok = ratio >= threshold
        results.append(
            {
                "label": label,
                "fg": f"{fg}={tokens[fg]}",
                "bg": f"{bg}={tokens[bg]}",
                "ratio": round(ratio, 2),
                "threshold": threshold,
                "kind": kind,
                "pass": ok,
            }
        )
        if not ok:
            failures.append(f"{label} ratio={ratio:.2f} < {threshold}")

    print(json.dumps({"reference_white_on_black": round(reference, 2), "pairs": results}, indent=1))
    if failures:
        for failure in failures:
            print(f"FAIL_CONTRAST {failure}")
        return 1
    print("PASS_CONTRAST_WCAG_AA")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
