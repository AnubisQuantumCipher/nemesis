import { useState } from "react";

import type { NavigationName } from "../data/navigation";
import type {
  CommandFailure,
  CompiledMission,
  DesktopSettings,
  MissionDraftRequest,
  MissionRunResult,
  MissionRuntimeSnapshot,
  ReplayResult,
  SystemStatus,
} from "../lib/bridge";
import { EvidencePanel } from "./EvidencePanel";
import { ReplayPanel } from "./ReplayPanel";
import { SettingsPanel } from "./SettingsPanel";
import { StatusMarker } from "./StatusMarker";

interface MissionCockpitProps {
  selected: NavigationName;
  system: SystemStatus;
  contractPath: string;
  compiled: CompiledMission | null;
  runtime: MissionRuntimeSnapshot;
  result: MissionRunResult | null;
  replay: ReplayResult | null;
  replayLoading: boolean;
  compiling: boolean;
  drafting: boolean;
  savingSettings: boolean;
  error: CommandFailure | null;
  onSelect: (name: NavigationName) => void;
  onContractPath: (path: string) => void;
  onCompile: () => void;
  onDraft: (request: MissionDraftRequest) => Promise<void>;
  onReview: () => void;
  onCancel: () => void;
  onSaveSettings: (settings: DesktopSettings) => Promise<void>;
}

function FailureNotice({ failure }: { failure: CommandFailure | null }) {
  if (!failure) {
    return null;
  }
  return (
    <div className="runtime-error" role="alert" tabIndex={-1}>
      <strong>{failure.code}</strong>
      <span>{failure.message}</span>
      <p>{failure.recovery}</p>
    </div>
  );
}

export function MissionCockpit({
  selected,
  system,
  contractPath,
  compiled,
  runtime,
  result,
  replay,
  replayLoading,
  compiling,
  drafting,
  savingSettings,
  error,
  onSelect,
  onContractPath,
  onCompile,
  onDraft,
  onReview,
  onCancel,
  onSaveSettings,
}: MissionCockpitProps) {
  const [draft, setDraft] = useState<MissionDraftRequest>({
    goal: "",
    workspace: "",
    relativePath: "",
    replacement: "",
  });
  if (selected === "Evidence") {
    return <EvidencePanel result={result} />;
  }
  if (selected === "Replay") {
    return <ReplayPanel replay={replay} loading={replayLoading} />;
  }
  if (selected === "Settings") {
    return (
      <SettingsPanel
        settings={system.settings}
        saving={savingSettings}
        onSave={onSaveSettings}
      />
    );
  }
  if (selected === "Home") {
    return (
      <section className="home-panel" aria-labelledby="home-heading">
        <header className="panel-heading">
          <div>
            <span className="section-index">LOCAL / FAIL-CLOSED</span>
            <h2 id="home-heading">Local control plane</h2>
          </div>
          <span className={system.ready ? "court-state is-verified" : "court-state"}>
            {system.ready ? "COMPONENTS READY" : "DEGRADED"}
          </span>
        </header>
        <div className="home-grid">
          <article>
            <span>CORE</span>
            <strong>{system.core}</strong>
            <p>Durable mission owner under local-home schema {system.schemaVersion}.</p>
          </article>
          <article>
            <span>KERNEL</span>
            <strong>{system.kernel}</strong>
            <p>Available is not a blanket proof claim.</p>
          </article>
          <article>
            <span>RUNTIME</span>
            <strong>{system.runtime}</strong>
            <p>{system.sandbox.replaceAll("_", " ")}</p>
          </article>
          <article>
            <span>NETWORK</span>
            <strong>DENIED</strong>
            <p>{system.network.replaceAll("_", " ")}</p>
          </article>
        </div>
        <div className="home-action">
          <div>
            <span>Last mission</span>
            <strong>{result ? result.missionId : "No completed local mission"}</strong>
          </div>
          <button type="button" className="primary-action" onClick={() => onSelect("Missions")}>
            Open missions
          </button>
        </div>
        <FailureNotice failure={error} />
      </section>
    );
  }

  return (
    <section className="mission-workspace" aria-labelledby="missions-heading">
      <header className="panel-heading">
        <div>
          <span className="section-index">MISSION / EXACT LOCAL CONTRACT</span>
          <h2 id="missions-heading">Local missions</h2>
        </div>
        <span className={runtime.running ? "live-indicator is-running" : "live-indicator"} aria-live="polite">
          {runtime.running ? runtime.phase : compiled ? "READY FOR REVIEW" : "NO CONTRACT"}
        </span>
      </header>

      <form
        className="contract-drafter"
        onSubmit={(event) => {
          event.preventDefault();
          void onDraft(draft);
        }}
      >
        <div className="drafter-heading">
          <div>
            <span className="section-index">CREATE / SOURCE-BOUND</span>
            <strong>Create an exact local contract</strong>
          </div>
          <p>NEMESIS reads the clean Git HEAD and target bytes, then writes a private draft under the local home.</p>
        </div>
        <div className="draft-fields">
          <label>
            <span>Mission goal</span>
            <input
              aria-label="Mission goal"
              value={draft.goal}
              onChange={(event) => setDraft((current) => ({ ...current, goal: event.target.value }))}
              disabled={drafting || runtime.running}
            />
          </label>
          <label>
            <span>Workspace path</span>
            <input
              aria-label="Workspace path"
              value={draft.workspace}
              onChange={(event) => setDraft((current) => ({ ...current, workspace: event.target.value }))}
              placeholder="/absolute/path/to/clean/git/repository"
              spellCheck={false}
              disabled={drafting || runtime.running}
            />
          </label>
          <label>
            <span>Relative file path</span>
            <input
              aria-label="Relative file path"
              value={draft.relativePath}
              onChange={(event) => setDraft((current) => ({ ...current, relativePath: event.target.value }))}
              placeholder="path/inside/repository.txt"
              spellCheck={false}
              disabled={drafting || runtime.running}
            />
          </label>
          <label>
            <span>Replacement UTF-8 text</span>
            <textarea
              aria-label="Replacement UTF-8 text"
              value={draft.replacement}
              onChange={(event) => setDraft((current) => ({ ...current, replacement: event.target.value }))}
              disabled={drafting || runtime.running}
            />
          </label>
        </div>
        <button
          type="submit"
          className="primary-action"
          disabled={
            drafting ||
            runtime.running ||
            Object.values(draft).some((value) => value.trim().length === 0)
          }
        >
          {drafting ? "Creating contract…" : "Create exact contract"}
        </button>
      </form>

      <div className="mission-production-grid">
        <article className="contract-composer">
          <label htmlFor="contract-path">Local mission contract path</label>
          <div className="contract-path-row">
            <input
              id="contract-path"
              type="text"
              value={contractPath}
              onChange={(event) => onContractPath(event.target.value)}
              placeholder="/absolute/path/to/mission.json"
              spellCheck={false}
              disabled={runtime.running}
            />
            <button
              type="button"
              className="review-button"
              onClick={onCompile}
              disabled={compiling || runtime.running || contractPath.trim().length === 0}
            >
              {compiling ? "Compiling…" : "Compile contract"}
            </button>
          </div>
          <p>
            The file must use <code>nemesis.desktop-mission/v1</code>. Duplicate keys, unknown
            fields, widened authority, stale source, dirty workspaces, traversal, and symlinks
            refuse before review.
          </p>
        </article>

        <article className="mission-progress" aria-live="polite" aria-atomic="true">
          <span>Current state</span>
          <strong>{runtime.phase}</strong>
          <p>{runtime.detail}</p>
          {runtime.running ? (
            <button type="button" className="danger-action" onClick={onCancel}>
              Cancel mission
            </button>
          ) : null}
        </article>
      </div>

      {compiled ? (
        <div className="compiled-contract">
          <div className="compiled-summary">
            <div>
              <span>GOAL</span>
              <strong>{compiled.goal}</strong>
            </div>
            <div>
              <span>WORKSPACE / FILE</span>
              <strong>{compiled.workspace}</strong>
              <code>{compiled.relativePath}</code>
            </div>
            <div>
              <span>CAPABILITY</span>
              <strong>{compiled.capabilities.join(", ")}</strong>
              <p>{compiled.replacementBytes} bytes; network, secrets, push, and publish denied.</p>
            </div>
          </div>
          <div className="digest-grid">
            <div>
              <span>CONTRACT SHA-256</span>
              <code aria-label="Contract SHA-256 digest">{compiled.contractDigest}</code>
            </div>
            <div>
              <span>ACTION SHA-256</span>
              <code aria-label="Action SHA-256 digest">{compiled.actionDigest}</code>
            </div>
          </div>
          <button type="button" className="primary-action" onClick={onReview} disabled={runtime.running}>
            Review authority
          </button>
        </div>
      ) : (
        <div className="empty-state compact-empty">
          <strong>No compiled contract</strong>
          <p>No mission, worker, capability, claim, or number is inferred before compilation.</p>
        </div>
      )}

      {result ? (
        <div className="verified-banner">
          <span className="verified-sigil" aria-hidden="true">N</span>
          <div>
            <strong>MISSION VERIFIED</strong>
            <p>Kernel committed completion at event {result.sequence}; receipt tamper was rejected.</p>
          </div>
          <StatusMarker status="VERIFIED" compact />
        </div>
      ) : null}
      <FailureNotice failure={error} />
    </section>
  );
}
