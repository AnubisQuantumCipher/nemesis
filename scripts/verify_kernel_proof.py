#!/usr/bin/env python3
"""Validate GNATprove work, scope coverage, and source-bound proof metadata."""

from __future__ import annotations

import argparse
import dataclasses
import datetime as dt
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SUMMARY = ROOT / "build/obj/debug/gnatprove/gnatprove.out"
DEFAULT_SCOPE = ROOT / "config/formal-kernel-scope.json"
MAX_SUMMARY_BYTES = 5 * 1024 * 1024
UNIT_PATTERN = re.compile(r"^in unit ([a-z0-9-]+),", re.MULTILINE)
ANALYZED_PATTERN = re.compile(r"^Analyzed (\d+) units$", re.MULTILINE)
WARNING_PATTERN = re.compile(r"\((?:[^)]*?)(\d+) warnings and (\d+) pragma Assume statements\)")
TIMEOUT_PATTERN = re.compile(r"\b(?:timeout|timed out)\b", re.IGNORECASE)


class ProofValidationError(ValueError):
    pass


@dataclasses.dataclass(frozen=True)
class ProofSummary:
    total_obligations: int
    unproved: int
    analyzed_units: int
    units: set[str]
    warnings: int
    assumptions: int
    timeout_observed: bool


def _strict_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ProofValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def load_strict_json(text: str) -> Any:
    try:
        return json.loads(text, object_pairs_hook=_strict_object)
    except ProofValidationError:
        raise
    except json.JSONDecodeError as error:
        raise ProofValidationError(f"invalid JSON: {error}") from error


def _total_row(text: str) -> tuple[int, int]:
    for line in text.splitlines():
        fields = line.split()
        if fields and fields[0] == "Total" and len(fields) >= 2 and fields[1].isdigit():
            total = int(fields[1])
            final = fields[-1]
            unproved = 0 if final == "." else int(final) if final.isdigit() else -1
            return total, unproved
    raise ProofValidationError("GNATprove summary has no parseable Total row")


def parse_gnatprove_summary(text: str) -> ProofSummary:
    total, unproved = _total_row(text)
    analyzed_match = ANALYZED_PATTERN.search(text)
    if analyzed_match is None:
        raise ProofValidationError("GNATprove summary has no analyzed-unit count")
    warning_rows = WARNING_PATTERN.findall(text)
    return ProofSummary(
        total_obligations=total,
        unproved=unproved,
        analyzed_units=int(analyzed_match.group(1)),
        units=set(UNIT_PATTERN.findall(text)),
        warnings=sum(int(warnings) for warnings, _ in warning_rows),
        assumptions=sum(int(assumptions) for _, assumptions in warning_rows),
        timeout_observed=TIMEOUT_PATTERN.search(text) is not None,
    )


def validate_proof_summary(summary: ProofSummary, expected_units: set[str]) -> list[str]:
    errors: list[str] = []
    if summary.total_obligations <= 0:
        errors.append("proof reported zero obligations")
    if summary.unproved != 0:
        errors.append(f"proof reported unproved obligations={summary.unproved}")
    if summary.analyzed_units != len(expected_units):
        errors.append(
            "analyzed unit count contradicts scope: "
            f"observed={summary.analyzed_units} expected={len(expected_units)}"
        )
    if summary.units != expected_units:
        errors.append(
            "analyzed unit set contradicts scope: "
            f"missing={sorted(expected_units - summary.units)} "
            f"unexpected={sorted(summary.units - expected_units)}"
        )
    if summary.warnings:
        errors.append(f"proof reported warnings={summary.warnings}")
    if summary.assumptions:
        errors.append(f"proof reported pragma assumptions={summary.assumptions}")
    if summary.timeout_observed:
        errors.append("proof summary contains timeout evidence")
    return errors


def _safe_path(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = Path(value)
    return not path.is_absolute() and ".." not in path.parts


def validate_live_scope(scope: Any, root: Path = ROOT) -> list[str]:
    errors: list[str] = []
    if not isinstance(scope, dict) or scope.get("schema") != "nemesis.formal-kernel-scope/v1":
        return ["formal scope schema is invalid"]
    units = scope.get("proved_units")
    if not isinstance(units, dict) or not units:
        return ["proved_units must be a non-empty object"]
    live_bodies = {
        path.stem
        for path in (root / "kernel/src").glob("nemesis-kernel-*.adb")
        if path.is_file() and not path.is_symlink()
    }
    if set(units) != live_bodies:
        errors.append(
            "proved unit roster differs from live kernel bodies: "
            f"missing={sorted(live_bodies - set(units))} "
            f"unexpected={sorted(set(units) - live_bodies)}"
        )
    for unit, entry in units.items():
        prefix = f"proved_units.{unit}"
        if not isinstance(entry, dict):
            errors.append(f"{prefix} must be an object")
            continue
        operations = entry.get("public_operations")
        if (
            not isinstance(operations, list)
            or not operations
            or not all(isinstance(operation, str) and operation for operation in operations)
            or len(operations) != len(set(operations))
        ):
            errors.append(f"{prefix} public_operations is empty, malformed, or duplicated")
        for key in ("spec", "body"):
            relative = entry.get(key)
            if not _safe_path(relative):
                errors.append(f"{prefix}.{key} path is unsafe")
                continue
            path = root / relative
            try:
                path.lstat()
                text = path.read_text(encoding="utf-8")
            except OSError as error:
                errors.append(f"{prefix}.{key} unreadable={error}")
                continue
            if path.is_symlink() or not path.is_file():
                errors.append(f"{prefix}.{key} must be a regular non-symlink file")
            if "SPARK_Mode => On" not in text:
                errors.append(f"{prefix}.{key} does not declare SPARK_Mode => On")
    unproved = scope.get("unproved_authority_boundaries")
    if not isinstance(unproved, dict) or not unproved:
        errors.append("unproved_authority_boundaries must remain explicit")
    else:
        for unit, entry in unproved.items():
            prefix = f"unproved_authority_boundaries.{unit}"
            if not isinstance(entry, dict) or not entry.get("reason"):
                errors.append(f"{prefix} requires a reason")
                continue
            paths = entry.get("paths")
            if not isinstance(paths, list) or not paths:
                errors.append(f"{prefix} requires paths")
                continue
            for relative in paths:
                if not _safe_path(relative) or not (root / relative).is_file():
                    errors.append(f"{prefix} has missing or unsafe path={relative}")
    blockers = scope.get("trust_surface_blockers")
    resolutions = scope.get("trust_surface_resolutions", [])
    if not isinstance(blockers, list) or not isinstance(resolutions, list) or not all(
        isinstance(entry, dict) for entry in [*(blockers or []), *(resolutions or [])]
    ):
        errors.append("trust_surface_blockers and trust_surface_resolutions must be explicit lists")
    else:
        if not blockers and not resolutions:
            errors.append("trust surface state must remain explicit: no blockers and no resolutions")
        identifiers = [entry.get("id") for entry in [*blockers, *resolutions]]
        if len(identifiers) != len(set(identifiers)) or any(not value for value in identifiers):
            errors.append("trust_surface ids are empty or duplicated")
        for entry in resolutions:
            prefix = f"trust_surface_resolutions.{entry.get('id') or '?'}"
            if not entry.get("description") or not entry.get("resolution"):
                errors.append(f"{prefix} requires description and resolution")
            authorized = entry.get("authorized_by")
            if (
                not isinstance(authorized, dict)
                or not _safe_path(authorized.get("contract"))
                or not (root / authorized["contract"]).is_file()
                or not isinstance(authorized.get("contract_sha256"), str)
                or len(authorized.get("contract_sha256", "")) != 64
                or _sha256(root / authorized["contract"]) != authorized["contract_sha256"]
                or not authorized.get("date")
            ):
                errors.append(
                    f"{prefix} requires authorized_by contract path, matching contract_sha256, and date"
                )
    return errors


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _git(*arguments: str) -> str:
    result = subprocess.run(
        ["/usr/bin/git", *arguments],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return result.stdout.strip()


def _tool_version(command: list[str]) -> list[str] | str:
    result = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env={
            "HOME": str(Path.home()),
            "LANG": "C",
            "LC_ALL": "C",
            "PATH": "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin:"
            + str(Path.home() / ".alire/bin")
            + ":"
            + str(Path.home() / ".alire/libexec/spark/bin")
            + ":"
            + str(Path.home() / ".local/bin"),
        },
    )
    lines = [line.strip() for line in result.stdout.splitlines() if line.strip()]
    return lines if result.returncode == 0 and lines else f"UNAVAILABLE exit={result.returncode}"


def build_receipt(
    summary_path: Path,
    summary: ProofSummary,
    scope_path: Path,
    scope: dict[str, Any],
) -> dict[str, Any]:
    source_paths: set[str] = set()
    for entry in scope["proved_units"].values():
        source_paths.update((entry["spec"], entry["body"]))
    for entry in scope["unproved_authority_boundaries"].values():
        source_paths.update(entry["paths"])
    source_sha256 = {relative: _sha256(ROOT / relative) for relative in sorted(source_paths)}
    source_set_sha256 = hashlib.sha256(
        json.dumps(source_sha256, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    dirty = _git(
        "status",
        "--porcelain=v1",
        "--untracked-files=all",
        "--",
        "kernel",
        "daemon",
        "config/formal-kernel-scope.json",
        "scripts/prove_kernel.sh",
        "scripts/verify_kernel_proof.py",
    ).splitlines()
    production_formal_lane = (
        "BLOCKED_TRUST_SURFACE" if scope["trust_surface_blockers"] else "PASS_BOUNDED"
    )
    limitations = [
        "The bounded proof applies only to proved_units and their named obligations.",
        "SPARK conclusions do not extend to Rust, Tauri/WebKit, SQLite, Git, Keychain, macOS, compilers, provers, or dependencies.",
    ]
    if scope["trust_surface_blockers"]:
        limitations.append(
            "Unproved authority boundaries and trust-surface blockers keep the production formal lane non-PASS."
        )
    else:
        limitations.append(
            "Unproved authority boundaries remain SPARK_Mode Off and are covered by tests; "
            "resolved trust-surface changes are enforced by the daemon authority path and its hostile A-B-A gates."
        )
    return {
        "schema": "nemesis.kernel-proof/v1",
        "status": "PARTIAL",
        "bounded_proof_status": "PASS",
        "production_formal_lane": production_formal_lane,
        "subject": {
            "commit": _git("rev-parse", "HEAD"),
            "tree": _git("rev-parse", "HEAD^{tree}"),
            "dirty_paths": dirty,
            "source_set_sha256": source_set_sha256,
        },
        "command": "./scripts/prove_kernel.sh",
        "summary": {
            "path": str(summary_path.relative_to(ROOT)),
            "sha256": _sha256(summary_path),
            "bytes": summary_path.stat().st_size,
            "total_obligations": summary.total_obligations,
            "unproved": summary.unproved,
            "analyzed_units": summary.analyzed_units,
            "units": sorted(summary.units),
            "warnings": summary.warnings,
            "assumptions": summary.assumptions,
            "timeout_observed": summary.timeout_observed,
        },
        "scope": {
            "path": str(scope_path.relative_to(ROOT)),
            "sha256": _sha256(scope_path),
            "proved_units": scope["proved_units"],
            "unproved_authority_boundaries": scope["unproved_authority_boundaries"],
            "trust_surface_blockers": scope["trust_surface_blockers"],
            "trust_surface_resolutions": scope.get("trust_surface_resolutions", []),
        },
        "source_sha256": source_sha256,
        "toolchain": {
            "alire": _tool_version(["alr", "--version"]),
            "gnatprove_and_provers": _tool_version(
                ["alr", "exec", "--", "env", "-u", "LIBRARY_PATH", "gnatprove", "--version"]
            ),
            "gprbuild": _tool_version(
                ["alr", "exec", "--", "env", "-u", "LIBRARY_PATH", "gprbuild", "--version"]
            ),
        },
        "observed_at": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
        "limitations": limitations,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--summary", type=Path, default=DEFAULT_SUMMARY)
    parser.add_argument("--scope", type=Path, default=DEFAULT_SCOPE)
    parser.add_argument("--write", type=Path)
    arguments = parser.parse_args()
    summary_path = arguments.summary.resolve()
    scope_path = arguments.scope.resolve()
    try:
        if summary_path.is_symlink():
            raise ProofValidationError("GNATprove summary must not be a symlink")
        summary_bytes = summary_path.read_bytes()
        if not summary_bytes or len(summary_bytes) > MAX_SUMMARY_BYTES:
            raise ProofValidationError("GNATprove summary is empty or oversized")
        summary_text = summary_bytes.decode("utf-8")
        scope = load_strict_json(scope_path.read_text(encoding="utf-8"))
        summary = parse_gnatprove_summary(summary_text)
    except (OSError, UnicodeDecodeError, ProofValidationError) as error:
        print(f"FAIL_KERNEL_PROOF_MANIFEST unreadable={error}")
        return 1
    errors = validate_live_scope(scope, ROOT)
    errors.extend(validate_proof_summary(summary, set(scope["proved_units"])))
    if errors:
        for error in errors:
            print(f"FAIL_KERNEL_PROOF_MANIFEST {error}")
        return 1
    if arguments.write is not None:
        receipt = build_receipt(summary_path, summary, scope_path, scope)
        output = arguments.write.resolve()
        output.parent.mkdir(parents=True, exist_ok=True)
        temporary = output.with_name(f".{output.name}.tmp-{os.getpid()}")
        temporary.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        temporary.replace(output)
    lane = (
        "BLOCKED_TRUST_SURFACE" if scope["trust_surface_blockers"] else "PASS_BOUNDED"
    )
    print(
        "PASS_KERNEL_PROOF_MANIFEST "
        f"obligations={summary.total_obligations} units={summary.analyzed_units} "
        f"production_lane={lane}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
