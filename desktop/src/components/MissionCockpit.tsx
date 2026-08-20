import type { NavigationName } from "../data/navigation";
import type { MissionRunResult, ReplayResult, SystemStatus } from "../lib/bridge";
import { EvidencePanel } from "./EvidencePanel";
import { ReplayPanel } from "./ReplayPanel";
import { StatusMarker } from "./StatusMarker";

interface MissionCockpitProps {
  selected: NavigationName;
  system: SystemStatus | null;
  result: MissionRunResult | null;
  replay: ReplayResult | null;
  replayLoading: boolean;
  running: boolean;
  error: string | null;
  onReview: () => void;
}

const activity = [
  ["01", "CONTRACT", "Structured mission waits for exact authorization"],
  ["02", "LANE", "Disposable Git worktree; worker authority attenuated"],
  ["03", "VERIFY", "Final-source evidence required for every claim"],
  ["04", "RECEIPT", "Canonical COSE receipt; independent verifier"],
] as const;

const surfaceDescriptions: Partial<Record<NavigationName, string>> = {
  Home: "Mission alerts, local health, and authority pressure.",
  Workspaces: "Canonical repositories and isolated worker lanes.",
  Agents: "Untrusted workers, roles, health, and consequence ceilings.",
  Changes: "Lane diffs remain separate from canonical source.",
  Tests: "Deterministic checks and their exact source bindings.",
  Knowledge: "Provenance-aware facts, contradictions, and revocations.",
  Skills: "Proposed procedures staged before trust promotion.",
  Automations: "Local triggers stop at unapproved authority boundaries.",
  Integrations: "Providers expose tools; NEMESIS retains authority.",
  Security: "Capabilities, policies, approvals, secrets, and audit history.",
  Replay: "Recorded causality, not retrospective model storytelling.",
  Settings: "Local providers, storage, motion, and interface controls.",
};

export function MissionCockpit({
  selected,
  system,
  result,
  replay,
  replayLoading,
  running,
  error,
  onReview,
}: MissionCockpitProps) {
  if (selected === "Evidence") {
    return <EvidencePanel result={result} />;
  }
  if (selected === "Replay") {
    return <ReplayPanel replay={replay} loading={replayLoading} />;
  }

  const isMissionSurface = selected === "Missions" || selected === "Home";
  if (!isMissionSurface) {
    return (
      <section className="surface-placeholder" aria-labelledby="surface-heading">
        <span className="section-index">SURFACE / {selected.toUpperCase()}</span>
        <h2 id="surface-heading">{selected}</h2>
        <p>{surfaceDescriptions[selected]}</p>
        <div className="boundary-note">
          <strong>Local desktop boundary</strong>
          <span>This surface reads the same durable mission state. It does not widen authority.</span>
        </div>
      </section>
    );
  }

  return (
    <div className="cockpit-grid">
      <aside className="mission-rail" aria-label="Mission summary">
        <span className="section-index">MISSION / LOCAL-001</span>
        <h2>Witnessed local change</h2>
        <p className="mission-goal">
          Modify one isolated fixture, verify final source, survive Core restart, and reject receipt tampering.
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

        {result ? (
          <div className="verified-banner">
            <span className="verified-sigil" aria-hidden="true">N</span>
            <div>
              <strong>MISSION VERIFIED</strong>
              <p>Kernel accepted completion at event {result.sequence}. Receipt and tamper evidence are available.</p>
            </div>
          </div>
        ) : null}
        {error ? <p className="runtime-error" role="alert">{error}</p> : null}
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
          <code>{system?.contractSha256 ?? "Checking architect contract…"}</code>
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
