# Mission hygiene

NEMESIS built-in skill pack `mission-hygiene` v1.

A governed procedure for verifying that a completed mission left a defensible
evidence trail. Propose it through the Skills rail; every trust promotion is a
one-shot-reviewed mutation; the body below is content-addressed and verified
on every read.

## Procedure

1. Open **Evidence** and confirm the terminal state is `COMPLETE` with both
   claims `VERIFIED` and the tamper probe `REJECTED`. A missing tamper
   rejection is a finding, not a formality.
2. Open **Replay** and confirm `exact_state_reconstruction` is true and the
   model lane is explicitly comparative (`exact_model_reexecution` false).
   Replay that starts a daemon or calls a model is not replay.
3. Open **Changes** and confirm the mission's lane holds exactly the reviewed
   bytes and the canonical workspace HEAD did not move.
4. Open **Security** and confirm the mission's one-shot approval reads
   `CONSUMED` and its parent grant was never widened: the child capability in
   the ledger must carry the same scope digest with a shorter expiry.
5. If any check fails, revoke nothing, delete nothing: file the finding with
   the mission id, the ledger head, and the exact failing artifact path.
   Evidence is preserved, never repaired in place.

## Refusals

- Do not mark this procedure `trusted` without a named approver and a
  nonzero approval signature; the Skills rail refuses skipped states.
- Do not run hygiene against a mission whose receipts you did not read.
