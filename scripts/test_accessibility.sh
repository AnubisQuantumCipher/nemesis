#!/usr/bin/env bash
# Phase G — accessibility and native-usability gate.
# Automatable lanes: WCAG contrast (G-03), reduced-motion wiring (G-05),
# color-independent status + keyboard reachability + accessible names via the
# DOM contract tests (G-01/G-02/G-06/G-09). The native macOS accessibility-tree
# and keyboard walkthrough (G-10) require an unlocked window-server session and
# are captured separately by scripts/capture_macos_accessibility.swift; when the
# console is locked that lane is recorded as an environment blocker, never faked.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# G-03 contrast instrument (fails closed below WCAG thresholds).
python3 scripts/measure_contrast.py >/dev/null

# G-05 reduced-motion must be honored both by explicit setting and OS preference.
grep -q 'prefers-reduced-motion: reduce' desktop/src/styles.css
grep -q 'data-reduce-motion="true"' desktop/src/styles.css

# G-01/G-02/G-06/G-09 DOM contract tests (focus, roles/labels, color-independent status).
if [[ ! -d desktop/node_modules ]]; then
  npm --prefix desktop ci
fi
npm --prefix desktop test

printf '%s\n' 'PASS_PRODUCTION_ACCESSIBILITY'
