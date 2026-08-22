#!/usr/bin/env bash
# Phase H — controlled performance and resource gate.
# Validates instruments, then measures daemon control-plane launch latency,
# idle RSS, mission-start (create) latency, authorize/run transition, bounded
# evidence growth, declared product limits, and package/binary sizes. Fails
# closed on a missing declared limit or a latency regression past its justified
# threshold. GUI window-paint launch is recorded as an environment blocker when
# the console is locked; it is never faked.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

python3 scripts/measure_performance.py

printf '%s\n' 'PASS_PRODUCTION_PERFORMANCE'
