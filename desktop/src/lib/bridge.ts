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
