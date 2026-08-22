#!/usr/bin/env python3
"""Run the bounded NEMESIS Desktop witnessed-mission backend slice."""

from __future__ import annotations

import argparse
import hashlib
import json
import socket
import os
import shutil
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]


def executable_paths(root: Path, prebuilt: Path | None) -> dict[str, Path]:
    if prebuilt is not None:
        return {
            "daemon": prebuilt / "nemesis_core_daemon",
            "lane_create": prebuilt / "nemesis-lane-create",
            "worker_runner": prebuilt / "nemesis-worker-runner",
            "worker": prebuilt / "nemesis-deterministic-worker",
            "signer": prebuilt / "nemesis-signer",
            "verifier": prebuilt / "nemesis-verify",
        }
    runtime = root / "runtime/target/debug"
    return {
        "daemon": root / "build/bin/nemesis_core_daemon",
        "lane_create": runtime / "nemesis-lane-create",
        "worker_runner": runtime / "nemesis-worker-runner",
        "worker": runtime / "nemesis-deterministic-worker",
        "signer": runtime / "nemesis-signer",
        "verifier": runtime / "nemesis-verify",
    }


prebuilt_value = os.environ.get("NEMESIS_PREBUILT_BIN_DIR")
paths = executable_paths(ROOT, Path(prebuilt_value) if prebuilt_value else None)
DAEMON = paths["daemon"]
LANE_CREATE = paths["lane_create"]
WORKER_RUNNER = paths["worker_runner"]
WORKER = paths["worker"]
SIGNER = paths["signer"]
VERIFIER = paths["verifier"]
GIT = Path("/usr/bin/git")

MISSION_ID = "mis_0000000000000000000000"
WORKER_ID = "wrk_0000000000000000000000"
GRANT_ID = "cap_" + MISSION_ID[4:]
APPROVAL_ID = "apr_" + MISSION_ID[4:]


def run(command: list[str | Path], *, cwd: Path | None = None) -> subprocess.CompletedProcess[bytes]:
    result = subprocess.run(
        [str(item) for item in command],
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"command failed ({result.returncode}): {' '.join(map(str, command))}\n"
            f"stdout={result.stdout.decode(errors='replace')}\n"
            f"stderr={result.stderr.decode(errors='replace')}"
        )
    return result


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def source_digest(repo: Path, lane: Path) -> str:
    base = run([GIT, "rev-parse", "HEAD"], cwd=repo).stdout.strip()
    value = (lane / "value.txt").read_bytes()
    return sha256(base + b"\0value.txt\0" + value)


class Core:
    def __init__(self, home: Path) -> None:
        self.home = home
        self.socket_path = home / "core.sock"
        self.process: subprocess.Popen[bytes] | None = None

    def start(self) -> None:
        self.home.mkdir(parents=True, exist_ok=True)
        self.process = subprocess.Popen(
            [str(DAEMON), "--home", str(self.home)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                stdout, stderr = self.process.communicate()
                raise RuntimeError(
                    f"Core exited {self.process.returncode}: "
                    f"{stdout.decode()} {stderr.decode()}"
                )
            if self.socket_path.exists():
                try:
                    response = self.request("ping")
                except (ConnectionRefusedError, FileNotFoundError, RuntimeError):
                    pass
                else:
                    if response.get("status") == "OK":
                        return
            time.sleep(0.01)
        raise RuntimeError("Core Unix socket did not become ready")

    def stop(self) -> None:
        if self.process is None:
            return
        if self.process.poll() is None:
            self.process.terminate()
        self.process.communicate(timeout=5)
        self.process = None

    def restart(self) -> None:
        self.stop()
        self.start()

    def request(self, command: str, **fields: object) -> dict[str, Any]:
        payload = {
            "schema": "nemesis.local/v1",
            "command": command,
            **fields,
        }
        data = canonical(payload) + b"\n"
        if len(data) > 65_536:
            raise RuntimeError("local request exceeded protocol limit")
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.connect(str(self.socket_path))
            client.sendall(data)
            response = bytearray()
            while not response.endswith(b"\n"):
                chunk = client.recv(4096)
                if not chunk:
                    raise RuntimeError("Core closed a response before LF")
                response.extend(chunk)
                if len(response) > 65_536:
                    raise RuntimeError("Core response exceeded protocol limit")
        parsed = json.loads(response)
        if not isinstance(parsed, dict):
            raise RuntimeError("Core response was not a JSON object")
        return parsed


def expect(response: dict[str, Any], status: str = "OK") -> dict[str, Any]:
    if response.get("status") != status:
        raise RuntimeError(f"unexpected Core response: {response}")
    return response


def initialize_fixture(directory: Path) -> tuple[Path, str]:
    repo = directory / "repository"
    repo.mkdir()
    run([GIT, "init", "-b", "main"], cwd=repo)
    (repo / "value.txt").write_text("before\n", encoding="utf-8")
    run([GIT, "add", "value.txt"], cwd=repo)
    run(
        [
            GIT,
            "-c",
            "user.name=NEMESIS Fixture",
            "-c",
            "user.email=nemesis-fixture@invalid",
            "commit",
            "-m",
            "fixture",
        ],
        cwd=repo,
    )
    base = run([GIT, "rev-parse", "HEAD"], cwd=repo).stdout.decode().strip()
    return repo, base


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    output = arguments.output.resolve()
    output.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="nemesis-vertical-slice-") as temporary:
        temporary_root = Path(temporary)
        repo, base_revision = initialize_fixture(temporary_root)
        lane = temporary_root / "lane"
        lane_result = json.loads(
            run(
                [LANE_CREATE, repo, lane, "nemesis/vertical-slice"],
            ).stdout
        )
        if lane_result.get("status") != "CREATED":
            raise RuntimeError(f"lane creation failed: {lane_result}")
        if (repo / "value.txt").read_text(encoding="utf-8") != "before\n":
            raise RuntimeError("canonical repository changed during lane creation")

        initial_source = source_digest(repo, lane)
        scope_digest = sha256(str(lane.resolve()).encode())
        contract = {
            "schema": "nemesis.mission/v1",
            "mission_id": MISSION_ID,
            "workspace": str(repo.resolve()),
            "base_revision": base_revision,
            "lane": str(lane.resolve()),
            "invariants": {
                "default_branch_unchanged": True,
                "push": False,
                "publish": False,
                "worker_state_mutation": False,
            },
            "completion": ["build", "tests"],
        }
        contract_digest = sha256(canonical(contract))
        desired = b"after\n"
        content_digest = sha256(desired)
        normalized_action = {
            "kind": "filesystem.modify",
            "relative_path": "value.txt",
            "content_digest": content_digest,
        }
        action_digest = sha256(canonical(normalized_action))

        core = Core(temporary_root / "nemesis-home")
        core.start()
        try:
            expect(
                core.request(
                    "create",
                    mission_id=MISSION_ID,
                    worker_id=WORKER_ID,
                    contract_digest=contract_digest,
                    scope_digest=scope_digest,
                    source_digest=initial_source,
                )
            )
            expect(
                core.request(
                    "authorize",
                    mission_id=MISSION_ID,
                    contract_digest=contract_digest,
                )
            )
            expect(
                core.request(
                    "create_grant",
                    mission_id=MISSION_ID,
                    grant_id=GRANT_ID,
                )
            )
            expect(
                core.request(
                    "create_approval",
                    mission_id=MISSION_ID,
                    approval_id=APPROVAL_ID,
                    action_digest=action_digest,
                )
            )
            expect(core.request("run", mission_id=MISSION_ID))

            worker = run(
                [
                    WORKER_RUNNER,
                    WORKER,
                    lane,
                    "plan",
                    "--mission-id",
                    MISSION_ID,
                    "--worker-id",
                    WORKER_ID,
                    "--path",
                    "value.txt",
                    "--content-digest",
                    content_digest,
                ]
            )
            proposals = [json.loads(line) for line in worker.stdout.splitlines()]
            if [message.get("method") for message in proposals] != [
                "heartbeat",
                "propose_action",
            ]:
                raise RuntimeError(f"unexpected worker protocol: {proposals}")
            if proposals[1]["params"] != {
                "action_kind": "filesystem.modify",
                "content_digest": content_digest,
                "purpose": "Apply the authorized deterministic fixture change.",
                "relative_path": "value.txt",
            }:
                raise RuntimeError("worker action was not the normalized mission action")

            action = expect(
                core.request(
                    "authorize_action",
                    mission_id=MISSION_ID,
                    worker_id=WORKER_ID,
                    scope_digest=scope_digest,
                    action_digest=action_digest,
                    estimated_bytes=len(desired),
                )
            )
            if action.get("decision") != "AUTHORIZED":
                raise RuntimeError(f"Kernel did not authorize exact action: {action}")

            sequence_before_restart = action["sequence"]
            core.restart()
            recovered = expect(core.request("inspect", mission_id=MISSION_ID))
            if recovered["sequence"] != sequence_before_restart or recovered["state"] != "RUNNING":
                raise RuntimeError("Core did not recover the acknowledged mission state")

            replayed = core.request(
                "authorize_action",
                mission_id=MISSION_ID,
                worker_id=WORKER_ID,
                scope_digest=scope_digest,
                action_digest=action_digest,
                estimated_bytes=len(desired),
            )
            if replayed.get("status") != "REFUSED" or replayed.get("reason") != "approval_replayed":
                raise RuntimeError(
                    f"one-shot approval was not durably consumed: {replayed}"
                )

            (lane / "value.txt").write_bytes(desired)
            final_source = source_digest(repo, lane)
            expect(
                core.request(
                    "action_completed",
                    mission_id=MISSION_ID,
                    source_digest=final_source,
                    action_digest=action_digest,
                )
            )

            build_check = run([GIT, "diff", "--check"], cwd=lane)
            test_check = run([WORKER_RUNNER, Path("/bin/cat"), lane, "value.txt"])
            if test_check.stdout != desired:
                raise RuntimeError("deterministic final-source test did not observe expected bytes")
            build_evidence = sha256(
                canonical(
                    {
                        "verifier": "/usr/bin/git diff --check",
                        "source_digest": final_source,
                        "exit_status": build_check.returncode,
                        "stdout_digest": sha256(build_check.stdout),
                        "stderr_digest": sha256(build_check.stderr),
                    }
                )
            )
            test_evidence = sha256(
                canonical(
                    {
                        "verifier": "/bin/cat value.txt under Workspace Safe",
                        "source_digest": final_source,
                        "exit_status": test_check.returncode,
                        "stdout_digest": sha256(test_check.stdout),
                        "stderr_digest": sha256(test_check.stderr),
                    }
                )
            )
            for claim, evidence in (("build", build_evidence), ("tests", test_evidence)):
                expect(
                    core.request(
                        "accept_evidence",
                        mission_id=MISSION_ID,
                        claim_id=claim,
                        source_digest=final_source,
                        evidence_digest=evidence,
                    )
                )
            completed = expect(core.request("propose_completion", mission_id=MISSION_ID))
            if completed.get("state") != "COMPLETE":
                raise RuntimeError(f"Kernel did not commit completion: {completed}")
            payload = expect(core.request("receipt_payload", mission_id=MISSION_ID))
            if payload["source_digest"] != final_source or not payload["claims_verified"]:
                raise RuntimeError("receipt payload was not final-source-bound")

            request = {
                "mission_id": MISSION_ID,
                "event_sequence": payload["event_sequence"],
                "terminal_state": payload["terminal_state"],
                "source_digest": payload["source_digest"],
                "ledger_head": payload["ledger_head"],
                "claims": [
                    {
                        "id": "build",
                        "mandatory": True,
                        "status": "VERIFIED",
                        "evidence_digest": payload["build_evidence"],
                    },
                    {
                        "id": "tests",
                        "mandatory": True,
                        "status": "VERIFIED",
                        "evidence_digest": payload["test_evidence"],
                    },
                ],
                "residuals": [
                    "Mobile, web, and public-release work remains DEFERRED.",
                    "The sandbox and external libraries remain documented assumptions.",
                ],
            }
            request_path = output / "receipt-request.json"
            receipt_path = output / "receipt.cose"
            public_key_path = output / "receipt.pub"
            request_path.write_bytes(canonical(request) + b"\n")
            signer = run(
                [
                    SIGNER,
                    "--request",
                    request_path,
                    "--receipt",
                    receipt_path,
                    "--public-key",
                    public_key_path,
                ]
            )
            signer_output = json.loads(signer.stdout)
            if signer_output.get("private_key_exported") is not False:
                raise RuntimeError("signer did not attest non-export")
            verified = run(
                [
                    VERIFIER,
                    "--receipt",
                    receipt_path,
                    "--public-key",
                    public_key_path,
                ]
            )
            verification = json.loads(verified.stdout)
            if verification.get("verdict") != "VERIFIED" or verification.get(
                "source_digest"
            ) != final_source:
                raise RuntimeError("standalone verifier rejected final receipt")

            tampered = bytearray(receipt_path.read_bytes())
            tampered[len(tampered) // 2] ^= 1
            tampered_path = output / "receipt-tampered.cose"
            tampered_path.write_bytes(tampered)
            rejected = subprocess.run(
                [
                    str(VERIFIER),
                    "--receipt",
                    str(tampered_path),
                    "--public-key",
                    str(public_key_path),
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            if rejected.returncode == 0 or json.loads(rejected.stdout).get("verdict") != "REJECTED":
                raise RuntimeError("one-byte receipt tamper was not rejected")

            core.restart()
            final = expect(core.request("inspect", mission_id=MISSION_ID))
            if final["state"] != "COMPLETE" or final["sequence"] != completed["sequence"]:
                raise RuntimeError("completed mission did not survive Core restart")
            if run([GIT, "rev-parse", "main"], cwd=repo).stdout.decode().strip() != base_revision:
                raise RuntimeError("default branch revision changed")
            if (repo / "value.txt").read_text(encoding="utf-8") != "before\n":
                raise RuntimeError("canonical repository file changed")

            shutil.copy2(
                core.home / "missions" / MISSION_ID / "events.ledger",
                output / "events.ledger",
            )
            (output / "contract.json").write_bytes(canonical(contract) + b"\n")
            (output / "verification.json").write_bytes(canonical(verification) + b"\n")
            (output / "daemon-final.json").write_bytes(canonical(final) + b"\n")
            (output / "source-digest.txt").write_text(final_source + "\n", encoding="utf-8")
        finally:
            core.stop()

    print("PASS_NEMESIS_BACKEND_VERTICAL_SLICE")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
