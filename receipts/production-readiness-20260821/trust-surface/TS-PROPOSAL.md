# Trust-Surface Change Proposal — TS-001 & TS-002 (UNMERGED)

**Status:** PROPOSED · UNMERGED · UNVERIFIED · architect sign-off REQUIRED
**Lane terminal state:** `BLOCKED_TRUST_SURFACE` for the daemon authority-integration lane.
**Subject commit:** `651e7d839bf1624df2c5e074c11a88a62f024e34`
**Author:** autonomous lead (no-human mission). This document is preserved as evidence and is deliberately **not applied**. Applying it changes an authorization accept condition and capability issuance semantics, which the mission forbids without exact architect sign-off.

## Why this is a trust surface, not an ordinary fix

The SPARK kernel already **proves** an exact, expiring, one-shot approval
(`Nemesis.Kernel.Approvals.Consume_Approval`, `kernel/src/nemesis-kernel-approvals.ads:29-42`)
and a parent→child attenuation relation
(`Nemesis.Kernel.Capabilities.Is_Attenuation`, `kernel/src/nemesis-kernel-capabilities.ads:65-79`).

The daemon's `authorize_action` handler does **not** use them. It synthesizes a
fresh grant in-line every request and calls only `Authorize`:

- Source: `daemon/src/nemesis_core_daemon.adb:225-243`
- Synthesized grant: fixed id `cap_0000…`, `Operations => [Modify_Data => True]`,
  `Maximum_Bytes => 4096`, `Expires_After => Current + 100`, `Status => Active`.
- `Approvals.Consume_Approval` is **never** called; there is no persisted parent
  grant and no `Is_Attenuation` check at the daemon boundary.

Changing this alters two trust surfaces:

- **TS-001** — Requiring a persisted, exact, one-shot approval to exist and be
  consumed inside `authorize_action` changes the **authorization accept
  condition**: today authorization succeeds whenever `Authorize(synth_grant,…)`
  returns `Authorized`; tomorrow it would additionally require a matching
  unconsumed approval record, and would mutate that record to consumed.
- **TS-002** — Replacing the synthesized grant with a persisted parent grant plus
  an attenuated child changes **capability issuance and replay semantics**
  (grants become durable objects with lineage, not per-request constants).

Both are recorded as architect-gated blockers in
`config/formal-kernel-scope.json:84-92` and `docs/architecture/FORMAL_ASSURANCE.md:45-46`.

## Proposed change (exact, unmerged)

The change requires a persisted approval + capability store in the daemon and a
new authorize path. The precise proposed transformation of the accept condition:

```
# daemon/src/nemesis_core_daemon.adb  authorize_action (currently :225-249)
# BEFORE (accept condition = Authorize on a synthesized grant):
   Grant := (Id => "cap_0000000000000000000000", ... Status => Active);
   Decision := Authorize (Grant, Action, Current);
   if Decision /= Authorized then return REFUSED; end if;
   Commit_Event (Home, Context, Action_Authorized, ...);

# AFTER (proposed; accept condition additionally requires a persisted, exact,
#        unconsumed, unexpired approval that is atomically consumed):
   Load_Parent_Grant (Home, Context.Id, Context.Worker, Parent, Load_Result);
   if Load_Result /= Store_OK then return REFUSED("no_parent_grant"); end if;
   Child := Derive_Child_Grant (Parent, Action);          -- must satisfy Is_Attenuation
   if not Is_Attenuation (Parent, Child) then return REFUSED("not_attenuated"); end if;
   Load_Approval (Home, Context.Id, Digest_256 (Action_Text), Appr, Load_Result);
   if Load_Result /= Store_OK then return REFUSED("no_approval"); end if;
   Consume_Approval (Appr, Context.Id, Digest_256 (Action_Text), Current, Consume_Result);
   if Consume_Result /= Approval_Consumed then
      return REFUSED("approval_" & Approval_Status'Image (Consume_Result));
   end if;
   Persist_Consumed_Approval (Home, Appr, Store_Result);  -- durable one-shot
   if Store_Result /= Store_OK then return REFUSED; end if;
   Decision := Authorize (Child, Action, Current);
   if Decision /= Authorized then return REFUSED; end if;
   Commit_Event (Home, Context, Action_Authorized, ...);
```

New durable store operations required (not yet implemented; must themselves be
crash-safe and fsync-ordered like `Nemesis.Core.Mission_Store`):
`Load_Parent_Grant`, `Persist_Grant`, `Load_Approval`, `Persist_Approval`,
`Persist_Consumed_Approval`, plus an approval-creation command on the local
protocol (`create_approval`) gated by explicit desktop authorization.

## Why it is not applied here

1. It changes the authorization accept condition and capability issuance
   semantics — explicitly architect-gated (mission "Trust-surface prohibition").
2. It requires new persisted authority state whose durability must be proven by
   new hostile/recovery tests; shipping it unverified would be fabricated
   assurance.
3. The no-human mission terminal state for this lane is therefore
   `BLOCKED_TRUST_SURFACE`; this proposal is preserved unmerged as the required
   evidence.

## Verification that would be required before merge (for the architect)

- SPARK proof that `Derive_Child_Grant` output always satisfies `Is_Attenuation`.
- Durability tests: approval consumed exactly once across crash/kill-during-commit.
- Negative controls: missing approval, expired approval, replayed approval,
  non-attenuated child, and mismatched action digest all REFUSE.
- A→B→A tamper: exact bytes, a compiling authority-widening variant rejected,
  byte-identical restoration, green rerun.
