import { invoke } from "@tauri-apps/api/core";

export type EvidenceStatus = "VERIFIED" | "BELIEVED" | "UNKNOWN";
export type TextScale = "standard" | "large";

export interface DesktopSettings {
  textScale: TextScale;
  reduceMotion: boolean;
}

export interface CommandFailure {
  code: string;
  message: string;
  recovery: string;
}

export interface ClaimResult {
  id: string;
  status: EvidenceStatus;
  evidenceDigest: string;
}

export interface MissionRunResult {
  status: "VERIFIED";
  missionId: string;
  sequence: number;
  sourceDigest: string;
  ledgerHead: string;
  artifactDirectory: string;
  lanePath: string;
  claims: ClaimResult[];
  tamperVerdict: "REJECTED";
  replayPath: string;
}

export interface CompiledMission {
  missionId: string;
  goal: string;
  workspace: string;
  baseRevision: string;
  relativePath: string;
  expectedSha256: string;
  contentDigest: string;
  replacementBytes: number;
  contractDigest: string;
  actionDigest: string;
  normalizedContract: string;
  capabilities: string[];
  maxWriteBytes: number;
  maxRuntimeSeconds: number;
  maxOutputBytes: number;
}

export interface MissionDraftRequest {
  goal: string;
  workspace: string;
  relativePath: string;
  replacement: string;
}

export interface DraftedMission {
  path: string;
  compiled: CompiledMission;
}

export interface SystemStatus {
  ready: boolean;
  core: string;
  kernel: string;
  runtime: string;
  contractSha256: string;
  localHome: string;
  firstLaunch: boolean;
  schemaVersion: number;
  appVersion: string;
  sandbox: string;
  network: string;
  updates: string;
  settings: DesktopSettings;
  lastMission: MissionRunResult | null;
}

export interface MissionRuntimeSnapshot {
  running: boolean;
  phase: string;
  detail: string;
  lastResult: MissionRunResult | null;
  error: CommandFailure | null;
}

export interface ReplayEvent {
  sequence: number;
  kindCode: number;
  stateCode: number;
  eventHash: string;
}

export interface ReplayResult {
  verdict: "VERIFIED";
  finalStateCode: number;
  head: string;
  exactStateReconstruction: boolean;
  exactModelReexecution: false;
  events: ReplayEvent[];
}

export interface RailMutationRequest {
  rail: string;
  verb: string;
  id: string;
  payload: Record<string, unknown>;
}

export interface RailEntityView {
  id: string;
  relativePath: string;
  sha256: string;
  bytes: number;
  value: Record<string, unknown>;
  derived?: Record<string, unknown>;
}

export interface RailEntitiesResponse {
  rail: string;
  entities: RailEntityView[];
  contradictions?: Array<Record<string, unknown>>;
  latestRuns?: Array<Record<string, unknown>>;
}

export interface AdoptionRecord {
  schema: string;
  sequence: number;
  missionId: string;
  rail: string;
  relativePath: string;
  contentDigest: string;
  contractDigest: string;
  actionDigest: string;
  stateCommit: string;
  previous: string;
  entryHash: string;
}

export interface ChainedReceipt {
  schema: string;
  sequence: number;
  subject: string;
  verdict: string;
  detail: Record<string, unknown>;
  recordedUnixSeconds: number;
  previous: string;
  entryHash: string;
}

export interface LaneChange {
  missionId: string;
  lanePath: string;
  status: string;
  branch?: string;
  head?: string;
  dirtyCount?: number;
  dirtyFiles?: Array<{ code: string; path: string }>;
  diffShortstat?: string;
  workspace?: string | null;
  baseRevision?: string | null;
  relativePath?: string | null;
  error?: string;
}

export interface ChangesSnapshot {
  laneCount: number;
  lanes: LaneChange[];
}

export interface SecuritySnapshot {
  policy: Record<string, string>;
  secretsPosture: Record<string, string>;
  grants: Array<Record<string, unknown>>;
  approvals: Array<Record<string, unknown>>;
  approvalTally: { armed: number; consumed: number; corrupt: number };
  adoptionChain: {
    length: number;
    head: string;
    verified: boolean;
    records: AdoptionRecord[];
  };
}

export function normalizeFailure(cause: unknown): CommandFailure {
  if (
    typeof cause === "object" &&
    cause !== null &&
    "code" in cause &&
    "message" in cause &&
    "recovery" in cause
  ) {
    const candidate = cause as Record<string, unknown>;
    if (
      typeof candidate.code === "string" &&
      typeof candidate.message === "string" &&
      typeof candidate.recovery === "string"
    ) {
      return {
        code: candidate.code,
        message: candidate.message,
        recovery: candidate.recovery,
      };
    }
  }
  return {
    code: "BLOCKED_DESKTOP_BRIDGE",
    message: cause instanceof Error ? cause.message : String(cause),
    recovery: "Quit and relaunch NEMESIS Desktop. No failed bridge call is treated as PASS.",
  };
}

export async function draftMission(request: MissionDraftRequest): Promise<DraftedMission> {
  return invoke<DraftedMission>("draft_mission", { request });
}

export async function compileMission(path: string): Promise<CompiledMission> {
  return invoke<CompiledMission>("compile_mission", { path });
}

export async function runMission(
  path: string,
  reviewedContractDigest: string,
  reviewedActionDigest: string,
): Promise<MissionRunResult> {
  return invoke<MissionRunResult>("run_mission", {
    path,
    reviewedContractDigest,
    reviewedActionDigest,
  });
}

export async function getMissionStatus(): Promise<MissionRuntimeSnapshot> {
  return invoke<MissionRuntimeSnapshot>("mission_status");
}

export async function cancelMission(): Promise<MissionRuntimeSnapshot> {
  return invoke<MissionRuntimeSnapshot>("cancel_mission");
}

export async function getReplay(): Promise<ReplayResult> {
  return invoke<ReplayResult>("load_replay");
}

export async function getSystemStatus(): Promise<SystemStatus> {
  return invoke<SystemStatus>("system_status");
}

export async function getSettings(): Promise<DesktopSettings> {
  return invoke<DesktopSettings>("get_settings");
}

export async function updateSettings(settings: DesktopSettings): Promise<DesktopSettings> {
  return invoke<DesktopSettings>("update_settings", { settings });
}

export async function railEntities(rail: string): Promise<RailEntitiesResponse> {
  return invoke<RailEntitiesResponse>("rail_entities", { rail });
}

export async function draftRailMutation(
  request: RailMutationRequest,
): Promise<DraftedMission> {
  return invoke<DraftedMission>("draft_rail_mutation", { request });
}

export async function adoptRailMutation(missionId: string): Promise<AdoptionRecord> {
  return invoke<AdoptionRecord>("adopt_rail_mutation", { missionId });
}

export async function changesSnapshot(): Promise<ChangesSnapshot> {
  return invoke<ChangesSnapshot>("changes_snapshot");
}

export async function securitySnapshot(): Promise<SecuritySnapshot> {
  return invoke<SecuritySnapshot>("security_snapshot");
}

export async function runRailTest(testId: string): Promise<ChainedReceipt> {
  return invoke<ChainedReceipt>("run_rail_test", { testId });
}

export async function dispatchAutomation(automationId: string): Promise<ChainedReceipt> {
  return invoke<ChainedReceipt>("dispatch_automation", { automationId });
}

export async function executeIntegrationPlugin(
  integrationId: string,
): Promise<ChainedReceipt> {
  return invoke<ChainedReceipt>("execute_integration_plugin", { integrationId });
}
