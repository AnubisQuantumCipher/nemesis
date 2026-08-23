/**
 * Authoritative kernel enum mirrors.
 *
 * These arrays MUST stay byte-for-byte aligned with the Ada source of truth.
 * Their array positions ARE the wire codes emitted by the replay verifier
 * (`runtime/crates/nemesis-runtime/src/bin/nemesis-replay.rs` -> `kind_code`
 * and `state_code`). Reordering an entry here silently mislabels the kernel
 * log; do not reorder.
 *
 * Mission state (state_code 0..10):
 *   kernel/src/nemesis-kernel-types.ads  ->  type Mission_State
 * Event kind (kind_code 0..9):
 *   daemon/src/nemesis-core-ledger.ads   ->  type Event_Kind
 */

/** kernel/src/nemesis-kernel-types.ads :: Mission_State (positional). */
export const MISSION_STATES = [
  "Draft",
  "Contract_Compiled",
  "Awaiting_Authorization",
  "Planning",
  "Running",
  "Waiting_Approval",
  "Recovering",
  "Verifying",
  "Complete",
  "Blocked_With_Evidence",
  "Cancelled",
] as const;

/** daemon/src/nemesis-core-ledger.ads :: Event_Kind (positional). */
export const EVENT_KINDS = [
  "Mission_Created",
  "Contract_Compiled_Event",
  "Contract_Authorized",
  "State_Transitioned",
  "Action_Authorized",
  "Action_Completed",
  "Evidence_Accepted",
  "Completion_Proposed",
  "Mission_Completed",
  "Mission_Blocked",
] as const;

/**
 * Human label for a mission `state_code`, spacing the Ada identifier, or a
 * truthful `STATE <code>` fallback when the daemon emits a code outside the
 * mirrored range. Shared by Activity, Conversation, and the evidence graph so
 * every surface names the same code identically.
 */
export function stateLabel(code: number): string {
  const name = MISSION_STATES[code];
  return name ? name.replaceAll("_", " ") : `STATE ${code}`;
}

/**
 * Human label for an event `kind_code`, spacing the Ada identifier, or a
 * truthful `KIND <code>` fallback when the daemon emits a code outside the
 * mirrored range.
 */
export function eventLabel(code: number): string {
  const name = EVENT_KINDS[code];
  return name ? name.replaceAll("_", " ") : `KIND ${code}`;
}
