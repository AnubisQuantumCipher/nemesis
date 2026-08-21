import { invoke } from "@tauri-apps/api/core";

export type EvidenceStatus = "VERIFIED" | "BELIEVED" | "UNKNOWN";

export interface SystemStatus {
  ready: boolean;
  core: string;
  kernel: string;
  runtime: string;
  contractSha256: string;
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
  claims: ClaimResult[];
  tamperVerdict: "REJECTED";
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
  exactModelReexecution: boolean;
  events: ReplayEvent[];
}

export async function getReplay(): Promise<ReplayResult> {
  return invoke<ReplayResult>("load_replay");
}

export async function getSystemStatus(): Promise<SystemStatus> {
  return invoke<SystemStatus>("system_status");
}

export async function runWitnessedMission(): Promise<MissionRunResult> {
  return invoke<MissionRunResult>("run_witnessed_mission");
}
