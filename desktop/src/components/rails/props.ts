import type { RailMutationRequest } from "../../lib/bridge";

/**
 * Shared contract for every rail panel.
 *
 * - `stateVersion` increments after every adopted rail mutation; panels
 *   MUST refetch when it changes.
 * - `busy` is true while a mission is drafting or running; mutation controls
 *   MUST be disabled.
 * - `onBeginMutation` drafts a governed mutation and opens the authority
 *   review surface; panels never mutate state through any other path.
 */
export interface RailPanelProps {
  stateVersion: number;
  busy: boolean;
  onBeginMutation: (request: RailMutationRequest) => void;
}
