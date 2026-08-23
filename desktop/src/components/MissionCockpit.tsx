import { useEffect, useState } from "react";

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
import { getReplay } from "../lib/bridge";
import { eventLabel, stateLabel } from "../data/kernelCodes";
import { EvidenceGraph } from "./EvidenceGraph";
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
  seedWorkspace?: string;
  onSeedConsumed?: () => void;
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
  seedWorkspace = "",
  onSeedConsumed,
}: MissionCockpitProps) {
  const [draft, setDraft] = useState<MissionDraftRequest>({
    goal: "",
    workspace: "",
    relativePath: "",
    replacement: "",
  });
  const [compose, setCompose] = useState(false);
  const [center, setCenter] = useState<"activity" | "diff" | "evidence" | "conversation">(
    "activity",
  );
  const [kernelLog, setKernelLog] = useState<ReplayResult | null>(replay);
  const [lastReplacement, setLastReplacement] = useState<string | null>(null);
  useEffect(() => {
    setKernelLog(replay);
  }, [replay]);
  useEffect(() => {
    if (seedWorkspace.length === 0) {
      return;
    }
    setCompose(true);
    setDraft((current) => ({ ...current, workspace: seedWorkspace }));
    onSeedConsumed?.();
  }, [seedWorkspace]);
  useEffect(() => {
    // Committed kernel events come from the last-mission ledger and are only
    // shown by the cockpit tabs (Home/Missions). Conversation always reads the
    // ledger; Activity only once a mission has completed (result set). Other
    // surfaces (Evidence/Replay/Settings) never load here, so a fresh cockpit
    // issues no load and never double-fetches with the Replay surface.
    const onCockpit = selected === "Home" || selected === "Missions";
    const shouldLoadEvents =
      onCockpit && (center === "conversation" || (center === "activity" && result !== null));
    if (!shouldLoadEvents) {
      return;
    }
    let current = true;
    getReplay()
      .then((loaded) => {
        if (current) {
          setKernelLog(loaded);
        }
      })
      .catch(() => {
        if (current && !replay) {
          setKernelLog(null);
        }
      });
    return () => {
      current = false;
    };
  }, [center, replay, result, selected]);
  if (selected === "Evidence") {
    return (
      <div className="evidence-surface">
        <EvidenceGraph result={result} />
        <EvidencePanel result={result} />
      </div>
    );
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
  if ((selected === "Home" || selected === "Missions") && !compose) {
    const running = runtime.running;
    const claims = result?.claims ?? [];
    const verifiedClaims = claims.filter((claim) => claim.status === "VERIFIED").length;
    // nemesis.desktop-mission/v1 requires exactly two completion predicates —
    // git_diff_check and content_match (production.rs::compile_local_contract).
    const requiredPredicates = 2;
    const predicateTotal = claims.length > 0 ? claims.length : requiredPredicates;
    const completionPercent =
      claims.length > 0 ? Math.round((verifiedClaims / claims.length) * 100) : 0;
    const events = kernelLog?.events ?? [];
    const sourceRevision = result?.sourceDigest ?? compiled?.baseRevision ?? null;
    const missionState = result
      ? "COMPLETE"
      : running
        ? runtime.phase
        : compiled
          ? "AWAITING AUTH"
          : "NO CONTRACT";
    const replacementByteLength =
      lastReplacement === null ? null : new TextEncoder().encode(lastReplacement).length;
    const replacementMatchesCompiled =
      compiled !== null &&
      replacementByteLength !== null &&
      replacementByteLength === compiled.replacementBytes;
    return (
      <div className="cockpit-grid">
        <aside className="mission-rail" aria-label="Mission summary">
          <span className="section-index">MISSION / LOCAL-001</span>
          <h2>Witnessed local change</h2>
          <p className="mission-goal">
            {compiled?.goal ??
              "Modify one isolated fixture, verify final source, survive Core restart, and reject receipt tampering."}
          </p>
          <dl className="mission-facts">
            <div>
              <dt>State</dt>
              <dd className={result ? "fact-verified" : running ? "fact-running" : ""}>
                {missionState}
              </dd>
            </div>
            <div>
              <dt>Completion</dt>
              <dd className={completionPercent === 100 ? "fact-verified" : ""}>
                {completionPercent}%
              </dd>
            </div>
            <div>
              <dt>Predicates</dt>
              <dd>
                {verifiedClaims} / {predicateTotal}
              </dd>
            </div>
            <div>
              <dt>Current phase</dt>
              <dd>{runtime.phase}</dd>
            </div>
            <div>
              <dt>Worker lane</dt>
              <dd>{result ? "1 isolated" : "1 on start"}</dd>
            </div>
            <div>
              <dt>Source revision</dt>
              <dd>{sourceRevision ? `${sourceRevision.slice(0, 12)}…` : "Pending"}</dd>
            </div>
            <div>
              <dt>Write budget</dt>
              <dd>{compiled ? `${compiled.maxWriteBytes} B` : "4096 B max"}</dd>
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
          <button
            type="button"
            className="review-button"
            onClick={() => {
              if (compiled) {
                onReview();
                return;
              }
              setCompose(true);
            }}
            disabled={running}
          >
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
              <h2 id="activity-heading">
                {center === "diff"
                  ? "Mission diff"
                  : center === "evidence"
                    ? "Mission evidence"
                    : center === "conversation"
                      ? "Kernel conversation"
                      : "Mission activity"}
              </h2>
            </div>
            <span className={running ? "live-indicator is-running" : "live-indicator"}>
              {result ? "SEALED" : running ? "EXECUTING" : "STANDBY"}
            </span>
          </header>
          <div className="cockpit-tabs" role="tablist" aria-label="Mission cockpit center">
            {(
              [
                ["activity", "Activity"],
                ["diff", "Diff"],
                ["evidence", "Evidence"],
                ["conversation", "Conversation"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                role="tab"
                aria-selected={center === id}
                className={center === id ? "cockpit-tab is-active" : "cockpit-tab"}
                onClick={() => setCenter(id)}
              >
                {label}
              </button>
            ))}
          </div>
          {center === "activity" ? (
            <div className="mission-graph" aria-label="Committed kernel activity">
              {running ? (
                <article className="graph-node is-active">
                  <span>LIVE</span>
                  <div>
                    <strong>{runtime.phase}</strong>
                    <p>{runtime.detail}</p>
                  </div>
                </article>
              ) : events.length > 0 ? (
                events.map((event, index) => (
                  <article
                    className={index === events.length - 1 ? "graph-node is-active" : "graph-node"}
                    key={event.sequence}
                  >
                    <span>{String(event.sequence).padStart(2, "0")}</span>
                    <div>
                      <strong>{eventLabel(event.kindCode)}</strong>
                      <p>{stateLabel(event.stateCode)}</p>
                      <code className="graph-hash">{event.eventHash}</code>
                    </div>
                  </article>
                ))
              ) : (
                <p className="graph-empty">
                  No committed kernel activity. Compile, review, and run a local contract to
                  populate the authoritative ledger.
                </p>
              )}
            </div>
          ) : null}
          {center === "diff" ? (
            <div className="cockpit-pane" aria-label="Mission diff">
              {compiled ? (
                <>
                  <dl className="mission-facts">
                    <div>
                      <dt>Workspace / file</dt>
                      <dd>{compiled.relativePath}</dd>
                    </div>
                    <div>
                      <dt>Action</dt>
                      <dd>replace_utf8 · byte-exact</dd>
                    </div>
                    <div>
                      <dt>Expected SHA-256 (before)</dt>
                      <dd>{compiled.expectedSha256}</dd>
                    </div>
                    <div>
                      <dt>Replacement bytes (after)</dt>
                      <dd>{compiled.replacementBytes}</dd>
                    </div>
                    <div>
                      <dt>Final source</dt>
                      <dd>{result?.sourceDigest ?? "Pending final source"}</dd>
                    </div>
                    <div>
                      <dt>Lane</dt>
                      <dd>{result?.lanePath ?? "No isolated lane yet"}</dd>
                    </div>
                  </dl>
                  {replacementMatchesCompiled && lastReplacement !== null ? (
                    <div className="diff-content">
                      <span className="section-index">
                        REPLACEMENT CONTENT / BYTE-EXACT · THIS SESSION
                      </span>
                      <pre aria-label="Replacement content">{lastReplacement}</pre>
                    </div>
                  ) : (
                    <p className="diff-note">
                      Byte-exact content preview is shown for contracts drafted in this session;
                      the change is otherwise bound only by the action digest above.
                    </p>
                  )}
                </>
              ) : (
                <p>No compiled contract. Diff is not inferred.</p>
              )}
            </div>
          ) : null}
          {center === "evidence" ? (
            <div className="cockpit-pane">
              <EvidenceGraph result={result} />
              <EvidencePanel result={result} />
            </div>
          ) : null}
          {center === "conversation" ? (
            <div className="cockpit-pane" aria-label="Kernel conversation">
              <p className="section-index">Kernel log, not model chat</p>
              {events.map((event) => (
                <article className="replay-event" key={event.sequence}>
                  <span className="replay-sequence">
                    EVENT {String(event.sequence).padStart(4, "0")}
                  </span>
                  <div>
                    <strong>
                      {eventLabel(event.kindCode)} · {stateLabel(event.stateCode)}
                    </strong>
                    <code>{event.eventHash}</code>
                  </div>
                </article>
              ))}
              {!replayLoading && events.length === 0 ? (
                <p>No committed kernel events. Workers do not speak here.</p>
              ) : null}
            </div>
          ) : null}
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
            <span>Durability</span>
            <strong>Daemon-owned durable state</strong>
            <p>
              The Core daemon persists a durable event ledger and recovers exact committed state
              across a full process restart — verified twice per run and reloaded on next launch.
            </p>
            <p className="inspector-gap">
              Gap: the in-flight orchestrator runs in this window. A mission interrupted by window
              close does not auto-resume; only committed state survives. Daemon-owned in-flight
              orchestration is a named residual.
            </p>
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
          <button type="button" className="live-indicator" onClick={() => setCompose(false)}>
            Return to cockpit
          </button>
        </div>
        <span className={runtime.running ? "live-indicator is-running" : "live-indicator"} aria-live="polite">
          {runtime.running ? runtime.phase : compiled ? "READY FOR REVIEW" : "NO CONTRACT"}
        </span>
      </header>

      <form
        className="contract-drafter"
        onSubmit={(event) => {
          event.preventDefault();
          setLastReplacement(draft.replacement);
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
