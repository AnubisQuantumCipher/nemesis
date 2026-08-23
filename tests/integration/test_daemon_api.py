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
GRANT = "cap_0000000000000000000000"
APPROVAL = "apr_0000000000000000000000"
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
        grant = self.request("create_grant", mission_id=MISSION, grant_id=GRANT)
        self.assertEqual(grant["status"], "OK")
        approval = self.request(
            "create_approval",
            mission_id=MISSION,
            approval_id=APPROVAL,
            action_digest=ACTION,
        )
        self.assertEqual(approval["status"], "OK")
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

    def test_authority_is_persisted_one_shot_and_fail_closed(self) -> None:
        def prepare(mission_id: str, *, grant: bool, approval: bool) -> None:
            created = self.request(
                "create",
                mission_id=mission_id,
                worker_id=WORKER,
                contract_digest=CONTRACT,
                scope_digest=SCOPE,
                source_digest=SOURCE_INITIAL,
            )
            self.assertEqual(created["status"], "OK")
            authorized = self.request(
                "authorize", mission_id=mission_id, contract_digest=CONTRACT
            )
            self.assertEqual(authorized["status"], "OK")
            if grant:
                issued = self.request(
                    "create_grant", mission_id=mission_id, grant_id=GRANT
                )
                self.assertEqual(issued["status"], "OK")
            if approval:
                issued = self.request(
                    "create_approval",
                    mission_id=mission_id,
                    approval_id=APPROVAL,
                    action_digest=ACTION,
                )
                self.assertEqual(issued["status"], "OK")

        def attempt(mission_id: str) -> dict[str, object]:
            return self.request(
                "authorize_action",
                mission_id=mission_id,
                worker_id=WORKER,
                scope_digest=SCOPE,
                action_digest=ACTION,
                estimated_bytes=128,
            )

        # (a) No grant and no approval: fail closed on the missing capability.
        bare = "mis_aaaaaaaaaaaaaaaaaaaaaa"
        prepare(bare, grant=False, approval=False)
        self.assertEqual(self.request("run", mission_id=bare)["status"], "OK")
        refused = attempt(bare)
        self.assertEqual(refused["status"], "REFUSED")
        self.assertEqual(refused["decision"], "REFUSED_CAPABILITY")
        self.assertEqual(refused["reason"], "no_parent_grant")

        # (e) Grant and approval issuance is a PLANNING-only authority.
        running_grant = self.request("create_grant", mission_id=bare, grant_id=GRANT)
        self.assertEqual(running_grant["status"], "REFUSED")
        running_approval = self.request(
            "create_approval",
            mission_id=bare,
            approval_id=APPROVAL,
            action_digest=ACTION,
        )
        self.assertEqual(running_approval["status"], "REFUSED")

        # (b) Grant present but no one-shot approval for the digest.
        unapproved = "mis_bbbbbbbbbbbbbbbbbbbbbb"
        prepare(unapproved, grant=True, approval=False)
        self.assertEqual(self.request("run", mission_id=unapproved)["status"], "OK")
        refused = attempt(unapproved)
        self.assertEqual(refused["status"], "REFUSED")
        self.assertEqual(refused["decision"], "REFUSED_APPROVAL")
        self.assertEqual(refused["reason"], "no_approval")

        # (c) Full path authorizes exactly once; the approval is consumed
        # durably before the authorization event and stays consumed across
        # a daemon kill/restart.
        oneshot = "mis_cccccccccccccccccccccc"
        prepare(oneshot, grant=True, approval=True)

        # (d) Duplicate issuance refuses without clobbering persisted authority.
        duplicate_grant = self.request(
            "create_grant", mission_id=oneshot, grant_id=GRANT
        )
        self.assertEqual(duplicate_grant["status"], "REFUSED")
        self.assertEqual(duplicate_grant["reason"], "grant_already_exists")
        duplicate_approval = self.request(
            "create_approval",
            mission_id=oneshot,
            approval_id=APPROVAL,
            action_digest=ACTION,
        )
        self.assertEqual(duplicate_approval["status"], "REFUSED")
        self.assertEqual(duplicate_approval["reason"], "approval_already_exists")

        self.assertEqual(self.request("run", mission_id=oneshot)["status"], "OK")
        authorized = attempt(oneshot)
        self.assertEqual(authorized["status"], "OK")
        self.assertEqual(authorized["decision"], "AUTHORIZED")
        replayed = attempt(oneshot)
        self.assertEqual(replayed["status"], "REFUSED")
        self.assertEqual(replayed["decision"], "REFUSED_APPROVAL")
        self.assertEqual(replayed["reason"], "approval_replayed")
        self.restart_daemon()
        replayed_after_restart = attempt(oneshot)
        self.assertEqual(replayed_after_restart["status"], "REFUSED")
        self.assertEqual(replayed_after_restart["decision"], "REFUSED_APPROVAL")
        self.assertEqual(replayed_after_restart["reason"], "approval_replayed")

        # (f) Hostile on-disk tamper of the approval record fails closed.
        tampered = "mis_dddddddddddddddddddddd"
        prepare(tampered, grant=True, approval=True)
        self.assertEqual(self.request("run", mission_id=tampered)["status"], "OK")
        approval_path = self.home / "missions" / tampered / f"approval-{ACTION}.apr"
        self.assertTrue(approval_path.exists())
        approval_path.write_bytes(b"\x00garbage-not-an-approval\xff")
        corrupt_approval = attempt(tampered)
        self.assertEqual(corrupt_approval["status"], "REFUSED")
        self.assertEqual(corrupt_approval["decision"], "REFUSED_APPROVAL")
        self.assertEqual(corrupt_approval["reason"], "approval_corrupt")

        # (f) Hostile on-disk tamper of the parent grant fails closed.
        broken = "mis_eeeeeeeeeeeeeeeeeeeeee"
        prepare(broken, grant=True, approval=True)
        self.assertEqual(self.request("run", mission_id=broken)["status"], "OK")
        grant_path = self.home / "missions" / broken / "parent.grant"
        self.assertTrue(grant_path.exists())
        grant_path.write_bytes(b"\x00garbage-not-a-grant\xff")
        corrupt_grant = attempt(broken)
        self.assertEqual(corrupt_grant["status"], "REFUSED")
        self.assertEqual(corrupt_grant["decision"], "REFUSED_CAPABILITY")
        self.assertEqual(corrupt_grant["reason"], "grant_corrupt")

    def test_capability_refusals_burn_the_one_shot_approval(self) -> None:
        #  PCL-03: exercise the post-consumption Authorize dimensions through
        #  the socket. Consumption precedes the capability check, so a scope,
        #  subject, or budget refusal burns the one-shot approval and an
        #  exact retry must report approval_replayed - the documented
        #  fail-closed direction, proven end to end.
        def prepare(mission_id: str) -> None:
            created = self.request(
                "create",
                mission_id=mission_id,
                worker_id=WORKER,
                contract_digest=CONTRACT,
                scope_digest=SCOPE,
                source_digest=SOURCE_INITIAL,
            )
            self.assertEqual(created["status"], "OK")
            self.assertEqual(
                self.request(
                    "authorize", mission_id=mission_id, contract_digest=CONTRACT
                )["status"],
                "OK",
            )
            self.assertEqual(
                self.request(
                    "create_grant", mission_id=mission_id, grant_id=GRANT
                )["status"],
                "OK",
            )
            self.assertEqual(
                self.request(
                    "create_approval",
                    mission_id=mission_id,
                    approval_id=APPROVAL,
                    action_digest=ACTION,
                )["status"],
                "OK",
            )
            self.assertEqual(
                self.request("run", mission_id=mission_id)["status"], "OK"
            )

        def exact_retry(mission_id: str) -> None:
            replayed = self.request(
                "authorize_action",
                mission_id=mission_id,
                worker_id=WORKER,
                scope_digest=SCOPE,
                action_digest=ACTION,
                estimated_bytes=128,
            )
            self.assertEqual(replayed["status"], "REFUSED")
            self.assertEqual(replayed["decision"], "REFUSED_APPROVAL")
            self.assertEqual(replayed["reason"], "approval_replayed")

        wrong_scope = "mis_ffffffffffffffffffffff"
        prepare(wrong_scope)
        refused = self.request(
            "authorize_action",
            mission_id=wrong_scope,
            worker_id=WORKER,
            scope_digest="99" * 32,
            action_digest=ACTION,
            estimated_bytes=128,
        )
        self.assertEqual(refused["status"], "REFUSED")
        self.assertEqual(refused["decision"], "REFUSED_CAPABILITY")
        self.assertNotIn("reason", refused)
        exact_retry(wrong_scope)

        wrong_worker = "mis_gggggggggggggggggggggg"
        prepare(wrong_worker)
        refused = self.request(
            "authorize_action",
            mission_id=wrong_worker,
            worker_id="wrk_1111111111111111111111",
            scope_digest=SCOPE,
            action_digest=ACTION,
            estimated_bytes=128,
        )
        self.assertEqual(refused["status"], "REFUSED")
        self.assertEqual(refused["decision"], "REFUSED_CAPABILITY")
        exact_retry(wrong_worker)

        over_budget = "mis_hhhhhhhhhhhhhhhhhhhhhh"
        prepare(over_budget)
        refused = self.request(
            "authorize_action",
            mission_id=over_budget,
            worker_id=WORKER,
            scope_digest=SCOPE,
            action_digest=ACTION,
            estimated_bytes=4_097,
        )
        self.assertEqual(refused["status"], "REFUSED")
        self.assertEqual(refused["decision"], "REFUSED_BUDGET")
        exact_retry(over_budget)

    def test_capability_check_petitions_without_burning_authority(self) -> None:
        # Worker-petition boundary. `capability_check` is the read-only kernel
        # adjudication a subprocess worker's proposal passes through before the
        # one-shot `authorize_action` is ever spent. It reuses the SPARK-proved
        # Load_Parent_Grant + Derive_Child_Grant + Authorize but loads no
        # approval, consumes nothing, and commits no event: an out-of-grant
        # proposal refuses REFUSED_CAPABILITY/REFUSED_BUDGET and an in-grant
        # proposal returns AUTHORIZED without advancing the ledger or burning
        # the one-shot approval. This is the tripwire the contract requires: if
        # create_grant is ever widened (subject, scope, or the 4096-byte
        # ceiling) the out-of-grant assertions below flip and this test fails.
        def prepare(mission_id: str, *, approval: bool) -> None:
            self.assertEqual(
                self.request(
                    "create",
                    mission_id=mission_id,
                    worker_id=WORKER,
                    contract_digest=CONTRACT,
                    scope_digest=SCOPE,
                    source_digest=SOURCE_INITIAL,
                )["status"],
                "OK",
            )
            self.assertEqual(
                self.request(
                    "authorize", mission_id=mission_id, contract_digest=CONTRACT
                )["status"],
                "OK",
            )
            self.assertEqual(
                self.request("create_grant", mission_id=mission_id, grant_id=GRANT)[
                    "status"
                ],
                "OK",
            )
            if approval:
                self.assertEqual(
                    self.request(
                        "create_approval",
                        mission_id=mission_id,
                        approval_id=APPROVAL,
                        action_digest=ACTION,
                    )["status"],
                    "OK",
                )
            self.assertEqual(self.request("run", mission_id=mission_id)["status"], "OK")

        def check(mission_id: str, **overrides: object) -> dict[str, object]:
            fields: dict[str, object] = dict(
                worker_id=WORKER,
                scope_digest=SCOPE,
                action_digest=ACTION,
                estimated_bytes=128,
            )
            fields.update(overrides)
            return self.request("capability_check", mission_id=mission_id, **fields)

        # A mission with a grant and a matching one-shot approval.
        approved = f"mis_{'p' * 22}"
        prepare(approved, approval=True)
        running_sequence = self.request("inspect", mission_id=approved)["sequence"]

        # In-grant proposal -> AUTHORIZED, and the ledger did not advance.
        granted = check(approved)
        self.assertEqual(granted["status"], "OK")
        self.assertEqual(granted["decision"], "AUTHORIZED")
        self.assertEqual(granted["sequence"], running_sequence)

        # Out-of-grant proposals -> the kernel refuses. Hostile twin + tripwire.
        wrong_subject = check(approved, worker_id=f"wrk_{'b' * 22}")
        self.assertEqual(wrong_subject["status"], "REFUSED")
        self.assertEqual(wrong_subject["decision"], "REFUSED_CAPABILITY")
        wrong_scope = check(approved, scope_digest="ab" * 32)
        self.assertEqual(wrong_scope["decision"], "REFUSED_CAPABILITY")
        over_budget = check(approved, estimated_bytes=4_097)
        self.assertEqual(over_budget["decision"], "REFUSED_BUDGET")

        # None of the read-only checks advanced the ledger.
        self.assertEqual(
            self.request("inspect", mission_id=approved)["sequence"], running_sequence
        )

        # The one-shot approval was NOT burned by capability_check: the real
        # authorize_action still consumes it exactly once.
        authorized = self.request(
            "authorize_action",
            mission_id=approved,
            worker_id=WORKER,
            scope_digest=SCOPE,
            action_digest=ACTION,
            estimated_bytes=128,
        )
        self.assertEqual(authorized["decision"], "AUTHORIZED")
        self.assertGreater(authorized["sequence"], running_sequence)

        # REQUIRES_APPROVAL: an in-grant proposal with no matching one-shot
        # approval is capability-AUTHORIZED but authorize_action refuses
        # no_approval - never a silent execute. The petition layer maps exactly
        # this pair (capability AUTHORIZED + REFUSED_APPROVAL/no_approval) to
        # REQUIRES_APPROVAL.
        unapproved = f"mis_{'q' * 22}"
        prepare(unapproved, approval=False)
        capable = check(unapproved)
        self.assertEqual(capable["decision"], "AUTHORIZED")
        needs = self.request(
            "authorize_action",
            mission_id=unapproved,
            worker_id=WORKER,
            scope_digest=SCOPE,
            action_digest=ACTION,
            estimated_bytes=128,
        )
        self.assertEqual(needs["status"], "REFUSED")
        self.assertEqual(needs["decision"], "REFUSED_APPROVAL")
        self.assertEqual(needs["reason"], "no_approval")

        # capability_check needs a RUNNING mission with a persisted grant;
        # without a grant it fails closed on the missing capability.
        bare = f"mis_{'r' * 22}"
        self.assertEqual(
            self.request(
                "create",
                mission_id=bare,
                worker_id=WORKER,
                contract_digest=CONTRACT,
                scope_digest=SCOPE,
                source_digest=SOURCE_INITIAL,
            )["status"],
            "OK",
        )
        self.assertEqual(
            self.request("authorize", mission_id=bare, contract_digest=CONTRACT)[
                "status"
            ],
            "OK",
        )
        self.assertEqual(self.request("run", mission_id=bare)["status"], "OK")
        missing = check(bare)
        self.assertEqual(missing["status"], "REFUSED")
        self.assertEqual(missing["decision"], "REFUSED_CAPABILITY")
        self.assertEqual(missing["reason"], "no_parent_grant")

    def test_stale_or_lagging_checkpoint_recovers_from_authoritative_ledger(self) -> None:
        # SQL-001: a crash between the durable ledger append and the checkpoint
        # rename leaves the checkpoint behind the ledger. The hash-chain-validated
        # ledger is authoritative, so the mission must still load (checkpoint
        # rebuilt from the ledger tail), never brick to STORE_CORRUPT.
        self.request(
            "create",
            mission_id=MISSION,
            worker_id=WORKER,
            contract_digest=CONTRACT,
            scope_digest=SCOPE,
            source_digest=SOURCE_INITIAL,
        )
        checkpoint = self.home / "missions" / MISSION / "checkpoint.ncp"
        stale = checkpoint.read_bytes()  # seq 2 snapshot

        authorized = self.request("authorize", mission_id=MISSION, contract_digest=CONTRACT)
        self.assertEqual(authorized["state"], "PLANNING")
        self.assertEqual(authorized["sequence"], 3)

        # Roll the checkpoint back to the stale snapshot; the ledger stays at seq 3.
        checkpoint.write_bytes(stale)
        recovered = self.request("inspect", mission_id=MISSION)
        self.assertEqual(recovered["status"], "OK")
        self.assertEqual(recovered["state"], "PLANNING")
        self.assertEqual(recovered["sequence"], 3)
        # The stale checkpoint must have been rewritten from the authoritative ledger.
        self.assertNotEqual(checkpoint.read_bytes(), stale)

        # A genuinely absent checkpoint also recovers from the ledger.
        checkpoint.unlink()
        self.restart_daemon()
        rebuilt = self.request("inspect", mission_id=MISSION)
        self.assertEqual(rebuilt["status"], "OK")
        self.assertEqual(rebuilt["sequence"], 3)
        self.assertTrue(checkpoint.exists())


if __name__ == "__main__":
    unittest.main()
