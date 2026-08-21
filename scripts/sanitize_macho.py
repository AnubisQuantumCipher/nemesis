#!/usr/bin/env python3
"""Remove private build RPATHs and reject non-system Mach-O dependencies."""

from __future__ import annotations

import argparse
import subprocess
from pathlib import Path


ALLOWED_DEPENDENCY_PREFIXES = (
    "/System/Library/",
    "/usr/lib/",
    "@executable_path/",
    "@loader_path/",
    "@rpath/",
)


def private_rpaths(output: str) -> list[str]:
    paths: list[str] = []
    lines = output.splitlines()
    for index, line in enumerate(lines):
        if line.strip() != "cmd LC_RPATH":
            continue
        for candidate in lines[index + 1 : index + 5]:
            stripped = candidate.strip()
            if not stripped.startswith("path "):
                continue
            value = stripped.removeprefix("path ").rsplit(" (offset ", 1)[0]
            if value.startswith(("/Users/", "/private/tmp/")):
                paths.append(value)
            break
    return paths


def dependencies(output: str) -> list[str]:
    values: list[str] = []
    for line in output.splitlines()[1:]:
        stripped = line.strip()
        if not stripped:
            continue
        values.append(stripped.split(" (compatibility version", 1)[0])
    return values


def command_output(*arguments: str | Path) -> str:
    return subprocess.run(
        [str(argument) for argument in arguments],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    ).stdout


def sanitize(path: Path) -> tuple[int, list[str]]:
    load_commands = command_output("/usr/bin/otool", "-l", path)
    removed = private_rpaths(load_commands)
    for rpath in removed:
        subprocess.run(
            ["/usr/bin/install_name_tool", "-delete_rpath", rpath, str(path)],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    remaining = private_rpaths(command_output("/usr/bin/otool", "-l", path))
    if remaining:
        raise RuntimeError("private RPATH remained after sanitization")
    unexpected = [
        dependency
        for dependency in dependencies(command_output("/usr/bin/otool", "-L", path))
        if not dependency.startswith(ALLOWED_DEPENDENCY_PREFIXES)
    ]
    return len(removed), unexpected


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("files", nargs="+", type=Path)
    arguments = parser.parse_args()
    removed_total = 0
    for path in arguments.files:
        try:
            removed, unexpected = sanitize(path)
        except (OSError, subprocess.CalledProcessError, RuntimeError) as error:
            print(f"FAIL_MACHO_SANITIZE path={path} error={type(error).__name__}")
            return 1
        removed_total += removed
        if unexpected:
            print(
                "FAIL_MACHO_DEPENDENCY "
                f"path={path} unexpected_count={len(unexpected)}"
            )
            return 1
    print(
        "PASS_MACHO_SANITIZE "
        f"files={len(arguments.files)} private_rpaths_removed={removed_total}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
