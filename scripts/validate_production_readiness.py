#!/usr/bin/env python3
"""Validate the fail-closed NEMESIS production-readiness roster."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from pathlib import Path
from typing import Any, Iterable

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "nemesis.production-readiness/v1"
REQUIRED_LANES = frozenset(
    {
        "function",
        "authority",
        "evidence_replay",
        "formal_kernel",
        "independent_autonomous_external_review",
        "dependency_supply_chain",
        "accessibility",
        "performance",
        "reliability_recovery",
        "install_upgrade_uninstall",
        "privacy",
        "packaging_signing_notarization",
        "hosted_ci",
        "release_readback",
        "supportability",
        "trust_surface",
    }
)
LANE_STATUSES = frozenset(
    {"PASS", "FAIL", "BLOCKED", "UNKNOWN", "INDETERMINATE", "SKIPPED"}
)
OVERALL_STATUSES = frozenset({"PASS", "FAIL", "BLOCKED", "INDETERMINATE"})
EVIDENCE_CLASSIFICATIONS = frozenset(
    {"CANONICAL", "FOCUSED", "VZ", "HOSTED", "BLOCKED"}
)
TERMINAL_VERDICTS = frozenset(
    {
        "COMPLETE_PRODUCTION_PUBLIC",
        "SEALED_LOCAL_PRODUCTION",
        "BLOCKED_PRODUCTION_PUBLIC",
        "BLOCKED_TRUST_SURFACE",
    }
)
HEX_40 = frozenset("0123456789abcdef")
HEX_64 = HEX_40


class ValidationError(ValueError):
    """Raised when JSON cannot be parsed without losing evidence."""


def _strict_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse_strict_json(text: str) -> Any:
    """Parse JSON while rejecting duplicate object keys at every depth."""

    try:
        return json.loads(text, object_pairs_hook=_strict_object)
    except ValidationError:
        raise
    except json.JSONDecodeError as error:
        raise ValidationError(f"invalid JSON: {error}") from error


def _hex(value: Any, length: int) -> bool:
    return (
        isinstance(value, str)
        and len(value) == length
        and all(character in HEX_40 for character in value)
    )


def _safe_relative(value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = Path(value)
    return not path.is_absolute() and ".." not in path.parts


def derive_overall(statuses: Iterable[str]) -> str:
    values = list(statuses)
    if not values or any(value not in LANE_STATUSES for value in values):
        return "INDETERMINATE"
    if "FAIL" in values:
        return "FAIL"
    if any(value in {"UNKNOWN", "INDETERMINATE", "SKIPPED"} for value in values):
        return "INDETERMINATE"
    if "BLOCKED" in values:
        return "BLOCKED"
    if all(value == "PASS" for value in values):
        return "PASS"
    return "INDETERMINATE"


def _validate_artifact(
    artifact: Any,
    marker: Any,
    root: Path,
    prefix: str,
    errors: list[str],
) -> None:
    if not isinstance(artifact, dict):
        errors.append(f"{prefix}: artifact must be an object")
        return
    relative = artifact.get("path")
    expected = artifact.get("sha256")
    if not _safe_relative(relative):
        errors.append(f"{prefix}: artifact path is unsafe")
        return
    candidate = root
    for part in Path(relative).parts:
        candidate /= part
        if candidate.is_symlink():
            errors.append(f"{prefix}: artifact must not be a symlink")
            return
    if not _hex(expected, 64):
        errors.append(f"{prefix}: artifact sha256 is invalid")
        return
    try:
        content = (root / relative).read_bytes()
    except OSError as error:
        errors.append(f"{prefix}: artifact unreadable: {error}")
        return
    actual = hashlib.sha256(content).hexdigest()
    if actual != expected:
        errors.append(f"{prefix}: artifact digest mismatch")
    if not isinstance(marker, str) or not marker.startswith("PASS_"):
        errors.append(f"{prefix}: terminal marker must start with PASS_")
        return
    try:
        lines = content.decode("utf-8").splitlines()
    except UnicodeDecodeError:
        errors.append(f"{prefix}: artifact is not UTF-8 marker evidence")
        return
    if not any(line == marker or line.startswith(f"{marker} ") for line in lines):
        errors.append(f"{prefix}: terminal marker missing from artifact")


def _validate_evidence(
    lane: dict[str, Any],
    subject: Any,
    root: Path,
    prefix: str,
    errors: list[str],
) -> None:
    evidence = lane.get("evidence")
    if not isinstance(evidence, list):
        errors.append(f"{prefix}: evidence must be an array")
        return
    if lane.get("status") == "PASS" and not evidence:
        errors.append(f"{prefix}: PASS requires evidence")
    seen: set[str] = set()
    for index, item in enumerate(evidence):
        item_prefix = f"{prefix} evidence[{index}]"
        if not isinstance(item, dict):
            errors.append(f"{item_prefix}: entry must be an object")
            continue
        evidence_id = item.get("id")
        if not isinstance(evidence_id, str) or not evidence_id:
            errors.append(f"{item_prefix}: id is required")
        elif evidence_id in seen:
            errors.append(f"{item_prefix}: duplicate evidence id {evidence_id}")
        else:
            seen.add(evidence_id)
        if item.get("classification") not in EVIDENCE_CLASSIFICATIONS:
            errors.append(f"{item_prefix}: classification is invalid")
        if not isinstance(item.get("command"), str) or not item.get("command"):
            errors.append(f"{item_prefix}: command is required")
        if type(item.get("exit_status")) is not int or item.get("exit_status") != 0:
            errors.append(f"{item_prefix}: accepted evidence requires exit_status 0")
        if item.get("subject") != subject:
            errors.append(f"{item_prefix}: evidence subject mismatch")
        _validate_artifact(
            item.get("artifact"),
            item.get("terminal_marker"),
            root,
            item_prefix,
            errors,
        )


def _validate_blockers(lane: dict[str, Any], prefix: str, errors: list[str]) -> None:
    blockers = lane.get("blockers")
    if not isinstance(blockers, list):
        errors.append(f"{prefix}: blockers must be an array")
        return
    if lane.get("status") == "BLOCKED" and not blockers:
        errors.append(f"{prefix}: BLOCKED requires blocker evidence")
    for index, blocker in enumerate(blockers):
        blocker_prefix = f"{prefix} blocker[{index}]"
        if not isinstance(blocker, dict):
            errors.append(f"{blocker_prefix}: entry must be an object")
            continue
        if not isinstance(blocker.get("id"), str) or not blocker.get("id"):
            errors.append(f"{blocker_prefix}: id is required")
        if blocker.get("kind") not in {"EXTERNAL", "TRUST_SURFACE"}:
            errors.append(f"{blocker_prefix}: kind is invalid")
        if not isinstance(blocker.get("command"), str) or not blocker.get("command"):
            errors.append(f"{blocker_prefix}: refused or failed command is required")
        if type(blocker.get("exit_status")) is not int:
            errors.append(f"{blocker_prefix}: exit_status is required")


def validate_roster(data: Any, root: Path = ROOT) -> list[str]:
    """Return every roster contradiction without short-circuiting."""

    errors: list[str] = []
    if not isinstance(data, dict):
        return ["roster must be a JSON object"]
    if data.get("schema") != SCHEMA:
        errors.append("schema is invalid")

    subject = data.get("subject")
    if not isinstance(subject, dict):
        errors.append("subject must be an object")
    else:
        if not _hex(subject.get("commit"), 40):
            errors.append("subject commit is invalid")
        if not _hex(subject.get("tree"), 40):
            errors.append("subject tree is invalid")

    lanes = data.get("lanes")
    lane_by_id: dict[str, dict[str, Any]] = {}
    statuses: list[str] = []
    if not isinstance(lanes, list) or not lanes:
        errors.append("lanes must be a non-empty array")
        lanes = []
    for index, lane in enumerate(lanes):
        prefix = f"lane[{index}]"
        if not isinstance(lane, dict):
            errors.append(f"{prefix}: lane must be an object")
            continue
        lane_id = lane.get("id")
        if not isinstance(lane_id, str) or not lane_id:
            errors.append(f"{prefix}: id is required")
        elif lane_id in lane_by_id:
            errors.append(f"{prefix}: duplicate lane id {lane_id}")
        else:
            lane_by_id[lane_id] = lane
        status = lane.get("status")
        if status not in LANE_STATUSES:
            errors.append(f"{prefix}: status is invalid")
        else:
            statuses.append(status)
        if lane_id in REQUIRED_LANES and lane.get("required") is not True:
            errors.append(f"{prefix}: required lane must set required=true")
        _validate_evidence(lane, subject, root, prefix, errors)
        _validate_blockers(lane, prefix, errors)
        if status == "FAIL" and not lane.get("findings"):
            errors.append(f"{prefix}: FAIL requires findings")

    missing = sorted(REQUIRED_LANES - set(lane_by_id))
    if missing:
        errors.append(f"required lane set missing={missing}")

    declared_overall = data.get("overall")
    derived_overall = derive_overall(statuses)
    if declared_overall not in OVERALL_STATUSES:
        errors.append("overall is invalid")
    elif declared_overall != derived_overall:
        errors.append(
            f"overall contradicts derived state: declared={declared_overall} derived={derived_overall}"
        )

    if type(data.get("automatable_complete")) is not bool:
        errors.append("automatable_complete must be boolean")
    external_blockers = data.get("external_blockers")
    if not isinstance(external_blockers, list):
        errors.append("external_blockers must be an array")
        external_blockers = []

    verdict = data.get("terminal_verdict")
    if verdict is not None and verdict not in TERMINAL_VERDICTS:
        errors.append("terminal_verdict is invalid")
    elif verdict == "COMPLETE_PRODUCTION_PUBLIC":
        if declared_overall != "PASS" or any(status != "PASS" for status in statuses):
            errors.append("COMPLETE_PRODUCTION_PUBLIC requires every lane PASS")
        if data.get("automatable_complete") is not True:
            errors.append("COMPLETE_PRODUCTION_PUBLIC requires automatable_complete=true")
        if external_blockers:
            errors.append("COMPLETE_PRODUCTION_PUBLIC forbids external blockers")
    elif verdict == "BLOCKED_PRODUCTION_PUBLIC":
        if declared_overall != "BLOCKED":
            errors.append("BLOCKED_PRODUCTION_PUBLIC requires overall BLOCKED")
        if data.get("automatable_complete") is not True:
            errors.append("BLOCKED_PRODUCTION_PUBLIC requires automatable_complete=true")
        if not external_blockers:
            errors.append("BLOCKED_PRODUCTION_PUBLIC requires external blockers")
        if any(
            not isinstance(blocker, dict) or blocker.get("kind") != "EXTERNAL"
            for blocker in external_blockers
        ):
            errors.append("BLOCKED_PRODUCTION_PUBLIC accepts EXTERNAL blockers only")
        if any(status not in {"PASS", "BLOCKED"} for status in statuses):
            errors.append("BLOCKED_PRODUCTION_PUBLIC forbids unfinished lane states")
    elif verdict == "BLOCKED_TRUST_SURFACE":
        trust_lane = lane_by_id.get("trust_surface", {})
        trust_blockers = trust_lane.get("blockers", []) if isinstance(trust_lane, dict) else []
        if declared_overall != "BLOCKED" or trust_lane.get("status") != "BLOCKED":
            errors.append("BLOCKED_TRUST_SURFACE requires blocked trust_surface lane")
        if data.get("automatable_complete") is not True:
            errors.append("BLOCKED_TRUST_SURFACE requires automatable_complete=true")
        if not any(
            isinstance(blocker, dict) and blocker.get("kind") == "TRUST_SURFACE"
            for blocker in trust_blockers
        ):
            errors.append("BLOCKED_TRUST_SURFACE requires TRUST_SURFACE blocker evidence")
        if any(status not in {"PASS", "BLOCKED"} for status in statuses):
            errors.append("BLOCKED_TRUST_SURFACE forbids unfinished lane states")
    elif verdict == "SEALED_LOCAL_PRODUCTION":
        if declared_overall != "PASS" or any(status != "PASS" for status in statuses):
            errors.append("SEALED_LOCAL_PRODUCTION requires every lane PASS")
        if data.get("automatable_complete") is not True:
            errors.append("SEALED_LOCAL_PRODUCTION requires automatable_complete=true")
        nonclaims = data.get("nonclaims")
        joined = " ".join(str(entry) for entry in nonclaims) if isinstance(nonclaims, list) else ""
        if (
            not isinstance(nonclaims, list)
            or "Developer ID" not in joined
            or "notariz" not in joined.lower()
            or "Gatekeeper" not in joined
        ):
            errors.append(
                "SEALED_LOCAL_PRODUCTION requires explicit Developer ID, notarization, "
                "and Gatekeeper nonclaims"
            )
        if any(
            not isinstance(blocker, dict)
            or blocker.get("kind") != "EXTERNAL"
            or blocker.get("required_for_verdict") is not False
            for blocker in external_blockers
        ):
            errors.append(
                "SEALED_LOCAL_PRODUCTION accepts only informational EXTERNAL blockers "
                "marked required_for_verdict=false"
            )

    if verdict is not None:
        for lane_id, lane in lane_by_id.items():
            lane_blockers = lane.get("blockers") or []
            if lane.get("status") == "PASS" and lane_blockers:
                errors.append(
                    f"terminal verdict forbids PASS lane {lane_id} carrying blockers"
                )
            if verdict != "BLOCKED_TRUST_SURFACE" and any(
                isinstance(blocker, dict) and blocker.get("kind") == "TRUST_SURFACE"
                for blocker in lane_blockers
            ):
                errors.append(
                    f"lane {lane_id} carries a TRUST_SURFACE blocker under verdict {verdict}"
                )

    if (
        verdict in {"SEALED_LOCAL_PRODUCTION", "COMPLETE_PRODUCTION_PUBLIC"}
        and isinstance(subject, dict)
        and (root / ".git").exists()
    ):
        commit = subject.get("commit", "")

        def _git(*arguments: str) -> subprocess.CompletedProcess[str]:
            return subprocess.run(
                ["/usr/bin/git", *arguments],
                cwd=root,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )

        if _git("cat-file", "-e", f"{commit}^{{commit}}").returncode != 0:
            errors.append("sealed roster subject commit is not in the repository")
        elif _git("merge-base", "--is-ancestor", commit, "HEAD").returncode != 0:
            errors.append("sealed roster subject commit is not an ancestor of HEAD")
        else:
            observed_tree = _git("rev-parse", f"{commit}^{{tree}}").stdout.strip()
            if subject.get("tree") != observed_tree:
                errors.append("sealed roster subject tree contradicts the commit")
            #  receipts/ and STATUS.md are post-seal evidence follow-up
            #  surfaces (v0.1.0 precedent); every other path is source-bound.
            changed = [
                line
                for line in _git(
                    "diff", "--name-only", f"{commit}..HEAD"
                ).stdout.splitlines()
                if line
                and not line.startswith("receipts/")
                and line != "STATUS.md"
            ]
            if changed:
                errors.append(
                    "sealed roster subject is stale: non-receipt paths changed "
                    f"since seal: {sorted(changed)[:8]}"
                )

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("roster", type=Path)
    parser.add_argument("--require-terminal", action="store_true")
    arguments = parser.parse_args()
    try:
        data = parse_strict_json(arguments.roster.read_text(encoding="utf-8"))
    except (OSError, ValidationError) as error:
        print(f"FAIL_PRODUCTION_ROSTER unreadable={error}")
        return 1
    errors = validate_roster(data, ROOT)
    if arguments.require_terminal and data.get("terminal_verdict") is None:
        errors.append("terminal verdict is required")
    if errors:
        for error in errors:
            print(f"FAIL_PRODUCTION_ROSTER {error}")
        return 1
    marker = "PASS_PRODUCTION_ROSTER_TERMINAL" if arguments.require_terminal else "PASS_PRODUCTION_ROSTER_VALID"
    print(
        f"{marker} overall={data['overall']} lanes={len(data['lanes'])} "
        f"verdict={data.get('terminal_verdict') or 'IN_PROGRESS'}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
