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
    const activity = [
      ["01", "CONTRACT", "Structured mission waits for exact authorization"],
      ["02", "LANE", "Disposable Git worktree; worker authority attenuated"],
      ["03", "VERIFY", "Final-source evidence required for every claim"],
      ["04", "RECEIPT", "Canonical COSE receipt; independent verifier"],
    ] as const;
    const running = runtime.running;
    return (
      <div className="cockpit-grid">
        <aside className="mission-rail" aria-label="Mission summary">
          <span className="section-index">MISSION / LOCAL-001</span>
          <h2>Witnessed local change</h2>
          <p className="mission-goal">
            Modify one isolated fixture, verify final source, survive Core restart, and reject receipt
            tampering.
          </p>
          <dl className="mission-facts">
            <div>
              <dt>State</dt>
              <dd className={result ? "fact-verified" : running ? "fact-running" : ""}>
                {result ? "COMPLETE" : running ? "RUNNING" : "AWAITING AUTH"}
              </dd>
            </div>
            <div>
              <dt>Predicates</dt>
              <dd>{result ? "2 / 2" : "0 / 2"}</dd>
            </div>
            <div>
              <dt>Worker lanes</dt>
              <dd>1 isolated</dd>
            </div>
            <div>
              <dt>Network</dt>
              <dd>Denied</dd>
            </div>
            <div>
              <dt>Push / publish</dt>
              <dd>Disabled</dd>
            </div>
          </dl>
          <button type="button" className="review-button" onClick={onReview} disabled={running}>
            Review contract
          </button>
          <div className="epistemic-legend" aria-label="Evidence legend">
            <StatusMarker status="VERIFIED" compact />
            <StatusMarker status="BELIEVED" compact />
            <StatusMarker status="UNKNOWN" compact />
          </div>
        </aside>
        <section className="activity-canvas" aria-labelledby="activity-heading">
          <header className="mission-header">
            <div>
              <span className="section-index">LIVE / CONTROL PLANE</span>
              <h2 id="activity-heading">Mission activity</h2>
            </div>
            <span className={running ? "live-indicator is-running" : "live-indicator"}>
              {result ? "SEALED" : running ? "EXECUTING" : "STANDBY"}
            </span>
          </header>
          <div className="mission-graph" aria-label="Mission dependency graph">
            {activity.map(([sequence, label, detail], index) => (
              <article
                className={result || index === 0 ? "graph-node is-active" : "graph-node"}
                key={sequence}
              >
                <span>{sequence}</span>
                <div>
                  <strong>{label}</strong>
                  <p>{detail}</p>
                </div>
              </article>
            ))}
          </div>
          {error ? (
            <div className="runtime-error" role="alert">
              <strong>{error.code}</strong>
              <span>{error.message}</span>
            </div>
          ) : null}
        </section>
        <aside className="inspector" aria-label="Selected object inspector">
          <header>
            <span className="section-index">INSPECTOR</span>
            <strong>{result ? "Receipt" : "Mission contract"}</strong>
          </header>
          <div className="inspector-section">
            <span>Authority source</span>
            <strong>SPARK Kernel</strong>
            <p>Workers propose. This pane reports committed decisions only.</p>
          </div>
          <div className="inspector-section">
            <span>Contract integrity</span>
            <code>{system.contractSha256 || "Checking architect contract…"}</code>
          </div>
          <div className="inspector-section">
            <span>Source identity</span>
            <code>{result?.sourceDigest ?? "Pending final source"}</code>
          </div>
          <div className="inspector-section">
            <span>Residuals</span>
            <p>Mobile, web, remote, packaging, and public release are DEFERRED.</p>
          </div>
        </aside>
      </div>
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
