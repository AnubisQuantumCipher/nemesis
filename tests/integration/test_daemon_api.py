from __future__ import annotations

import concurrent.futures
import json
import socket
import subprocess
import tempfile
import threading
import time
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DAEMON = ROOT / "build/bin/nemesis_core_daemon"
MISSION = "mis_0000000000000000000000"
WORKER = "wrk_0000000000000000000000"
CONTRACT = "11" * 32
SCOPE = "22" * 32
SOURCE_INITIAL = "33" * 32
SOURCE_FINAL = "44" * 32
SOURCE_CHANGED = "55" * 32
ACTION = "66" * 32
BUILD_EVIDENCE = "77" * 32
TEST_EVIDENCE = "88" * 32


class DaemonApiTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.home = Path(self.temporary.name) / "home"
        self.home.mkdir()
        self.socket_path = self.home / "core.sock"
        self.process: subprocess.Popen[bytes] | None = None
        self.start_daemon()

    def tearDown(self) -> None:
        self.stop_daemon()
        self.temporary.cleanup()

    def start_daemon(self) -> None:
        self.process = subprocess.Popen(
            [str(DAEMON), "--home", str(self.home)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if self.socket_path.exists():
                probe = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                try:
                    probe.connect(str(self.socket_path))
                except (ConnectionRefusedError, FileNotFoundError):
                    pass
                else:
                    probe.close()
                    return
                finally:
                    probe.close()
            if self.process.poll() is not None:
                stdout, stderr = self.process.communicate()
                self.fail(
                    f"daemon exited {self.process.returncode}: "
                    f"{stdout.decode()} {stderr.decode()}"
                )
            time.sleep(0.01)
        self.fail("daemon socket was not created")

    def stop_daemon(self) -> None:
        if self.process is None:
            return
        if self.process.poll() is None:
            self.process.terminate()
        self.process.communicate(timeout=5)
        self.process = None

    def restart_daemon(self) -> None:
        self.stop_daemon()
        self.start_daemon()

    def request(self, command: str, **fields: object) -> dict[str, object]:
        payload = {
            "schema": "nemesis.local/v1",
            "command": command,
            **fields,
        }
        encoded = json.dumps(payload, separators=(",", ":"), sort_keys=True).encode() + b"\n"
        self.assertLessEqual(len(encoded), 65_536)
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.connect(str(self.socket_path))
            client.sendall(encoded)
            response = bytearray()
            while not response.endswith(b"\n"):
                chunk = client.recv(4096)
                self.assertTrue(chunk, "daemon closed before newline response")
                response.extend(chunk)
                self.assertLessEqual(len(response), 65_536)
        return json.loads(response)

    def raw_request(self, chunks: list[bytes], *, shutdown: bool = False) -> dict[str, object]:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(2)
            client.connect(str(self.socket_path))
            for chunk in chunks:
                client.sendall(chunk)
            if shutdown:
                client.shutdown(socket.SHUT_WR)
            response = bytearray()
            while not response.endswith(b"\n"):
                part = client.recv(4096)
                self.assertTrue(part, "daemon closed before typed response")
                response.extend(part)
                self.assertLessEqual(len(response), 65_536)
        return json.loads(response)

    def test_protocol_handles_partial_reads_and_refuses_ambiguous_json(self) -> None:
        ping = b'{"command":"ping","schema":"nemesis.local/v1"}\n'
        partial = self.raw_request([ping[:13], ping[13:31], ping[31:]])
        self.assertEqual(partial["status"], "OK")

        duplicate = self.raw_request(
            [b'{"schema":"nemesis.local/v1","command":"ping","command":"create"}\n']
        )
        self.assertEqual(duplicate["status"], "INVALID_REQUEST")

        unknown = self.raw_request(
            [b'{"schema":"nemesis.local/v1","command":"ping","ambient":true}\n']
        )
        self.assertEqual(unknown["status"], "INVALID_REQUEST")

        incomplete = self.raw_request(
            [b'{"schema":"nemesis.local/v1","command":"ping"}'],
            shutdown=True,
        )
        self.assertEqual(incomplete["status"], "INVALID_REQUEST")

    def test_concurrent_create_has_one_authoritative_owner_and_no_duplicate_events(self) -> None:
        barrier = threading.Barrier(2)

        def create() -> dict[str, object]:
            barrier.wait()
            return self.request(
                "create",
                mission_id=MISSION,
                worker_id=WORKER,
                contract_digest=CONTRACT,
                scope_digest=SCOPE,
                source_digest=SOURCE_INITIAL,
            )

        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
            responses = list(executor.map(lambda _: create(), range(2)))

        self.assertEqual(sum(response["status"] == "OK" for response in responses), 1)
        self.assertEqual(sum(response["status"] != "OK" for response in responses), 1)
        inspected = self.request("inspect", mission_id=MISSION)
        self.assertEqual(inspected["sequence"], 2)
        ledger = self.home / "missions" / MISSION / "events.ledger"
        self.assertEqual(len(ledger.read_bytes().splitlines()), 2)

    def test_lifecycle_recovers_and_completion_rejects_stale_evidence(self) -> None:
        self.assertEqual(self.request("ping")["status"], "OK")
        created = self.request(
            "create",
            mission_id=MISSION,
            worker_id=WORKER,
            contract_digest=CONTRACT,
            scope_digest=SCOPE,
            source_digest=SOURCE_INITIAL,
        )
        self.assertEqual(created["state"], "AWAITING_AUTHORIZATION")
        self.assertEqual(created["sequence"], 2)

        wrong = self.request(
            "authorize", mission_id=MISSION, contract_digest="ff" * 32
        )
        self.assertEqual(wrong["status"], "REFUSED")
        authorized = self.request(
            "authorize", mission_id=MISSION, contract_digest=CONTRACT
        )
        self.assertEqual(authorized["state"], "PLANNING")
        running = self.request("run", mission_id=MISSION)
        self.assertEqual(running["state"], "RUNNING")

        action = self.request(
            "authorize_action",
            mission_id=MISSION,
            worker_id=WORKER,
            scope_digest=SCOPE,
            action_digest=ACTION,
            estimated_bytes=128,
        )
        self.assertEqual(action["decision"], "AUTHORIZED")
        sequence_before_restart = action["sequence"]

        self.restart_daemon()
        recovered = self.request("inspect", mission_id=MISSION)
        self.assertEqual(recovered["state"], "RUNNING")
        self.assertEqual(recovered["sequence"], sequence_before_restart)

        completed_action = self.request(
            "action_completed",
            mission_id=MISSION,
            source_digest=SOURCE_FINAL,
            action_digest=ACTION,
        )
        self.assertEqual(completed_action["status"], "OK")
        for claim, evidence in (("build", BUILD_EVIDENCE), ("tests", TEST_EVIDENCE)):
            accepted = self.request(
                "accept_evidence",
                mission_id=MISSION,
                claim_id=claim,
                source_digest=SOURCE_FINAL,
                evidence_digest=evidence,
            )
            self.assertEqual(accepted["status"], "OK")

        changed = self.request(
            "action_completed",
            mission_id=MISSION,
            source_digest=SOURCE_CHANGED,
            action_digest=ACTION,
        )
        self.assertEqual(changed["status"], "OK")
        stale = self.request("propose_completion", mission_id=MISSION)
        self.assertEqual(stale["status"], "EVIDENCE_STALE")
        self.assertEqual(stale["state"], "RUNNING")

        for claim, evidence in (("build", BUILD_EVIDENCE), ("tests", TEST_EVIDENCE)):
            self.request(
                "accept_evidence",
                mission_id=MISSION,
                claim_id=claim,
                source_digest=SOURCE_CHANGED,
                evidence_digest=evidence,
            )
        complete = self.request("propose_completion", mission_id=MISSION)
        self.assertEqual(complete["status"], "OK")
        self.assertEqual(complete["state"], "COMPLETE")

        receipt = self.request("receipt_payload", mission_id=MISSION)
        self.assertEqual(receipt["terminal_state"], "COMPLETE")
        self.assertEqual(receipt["source_digest"], SOURCE_CHANGED)
        self.assertEqual(receipt["claims_verified"], True)

        self.restart_daemon()
        final = self.request("inspect", mission_id=MISSION)
        self.assertEqual(final["state"], "COMPLETE")
        self.assertEqual(final["sequence"], complete["sequence"])

        terminal_retry = self.request(
            "authorize_action",
            mission_id=MISSION,
            worker_id=WORKER,
            scope_digest=SCOPE,
            action_digest=ACTION,
            estimated_bytes=1,
        )
        self.assertEqual(terminal_retry["status"], "REFUSED")
        unchanged = self.request("inspect", mission_id=MISSION)
        self.assertEqual(unchanged["sequence"], complete["sequence"])


if __name__ == "__main__":
    unittest.main()
