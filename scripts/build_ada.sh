#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Alire 2.1.0 prepends the selected GNAT runtime directory to LIBRARY_PATH.
# GNAT 14.2.1 already emits that runtime rpath on macOS. Keeping both creates
# duplicate LC_RPATH commands, which dyld on this host refuses at launch.
# Preserve Alire's selected PATH/GPR_PROJECT_PATH while removing only the
# duplicate linker input for the child gprbuild process.
exec alr exec -- env -u LIBRARY_PATH gprbuild -p -P "$ROOT/nemesis.gpr" "$@"
