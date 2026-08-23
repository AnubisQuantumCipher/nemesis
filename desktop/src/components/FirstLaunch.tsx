import { useState } from "react";

import type { SystemStatus } from "../lib/bridge";

// Surfaces the constitution (§4.1) enumerates but this local slice does not yet
// connect. Named as residuals — never rendered as selectable dummies.
const DEFERRED_LOCATIONS = ["Remote Mac / Linux server", "Connect to an existing daemon"];
const DEFERRED_WORKERS = [
  "Codex",
  "Claude Code",
  "OpenAI API",
  "Anthropic API",
  "OpenRouter",
  "Local model",
];
const DEFERRED_WORKSPACE_SOURCES = ["Clone a repository", "Connect GitHub", "Start without a repository"];

interface FirstLaunchProps {
  system: SystemStatus;
  onBegin: (workspace: string) => void;
}

export function FirstLaunch({ system, onBegin }: FirstLaunchProps) {
  const [workspace, setWorkspace] = useState("");
  const ready = system.ready;
  const components: ReadonlyArray<readonly [string, string, boolean]> = [
    ["Core", system.core, system.core === "READY"],
    ["Kernel", system.kernel, system.kernel === "AVAILABLE"],
    ["Runtime", system.runtime, system.runtime === "READY"],
  ];
  const unready = components.filter(([, , ok]) => !ok);
  const workspaceReady = workspace.trim().length > 0;
  const canBegin = ready && workspaceReady;

  return (
    <section className="first-launch" aria-labelledby="first-launch-heading">
      <span className="section-index">FIRST LAUNCH / LOCAL HOME · SCHEMA {system.schemaVersion}</span>
      <h2 id="first-launch-heading">Reach your first witnessed mission</h2>
      <p className="first-launch-lede">
        NEMESIS created private mission, lane, receipt, log, support, and temporary-data directories
        under the local home. No cloud account or telemetry endpoint was configured. Five steps take
        you to a real first mission — no terminal required.
      </p>
      <code className="first-launch-home" aria-label="NEMESIS local home path">
        {system.localHome}
      </code>

      <ol className="launch-steps">
        <li className="launch-step">
          <div className="launch-step-head">
            <span className="launch-step-index">1</span>
            <h3>Run location</h3>
          </div>
          <p className="launch-active">This Mac · local control plane</p>
          <p className="launch-residual">Deferred: {DEFERRED_LOCATIONS.join(" · ")}</p>
        </li>

        <li className="launch-step">
          <div className="launch-step-head">
            <span className="launch-step-index">2</span>
            <h3>Security profile</h3>
          </div>
          <p className="launch-active">Safe local development · enforced</p>
          <dl className="launch-facts">
            <div>
              <dt>Network</dt>
              <dd>{system.network.replaceAll("_", " ")}</dd>
            </div>
            <div>
              <dt>Updates</dt>
              <dd>{system.updates.replaceAll("_", " ")}</dd>
            </div>
            <div>
              <dt>Sandbox</dt>
              <dd>{system.sandbox.replaceAll("_", " ")}</dd>
            </div>
          </dl>
          <p className="launch-residual">
            Deferred: observe-only, sandboxed-autonomous, and advanced custom policies.
          </p>
        </li>

        <li className="launch-step">
          <div className="launch-step-head">
            <span className="launch-step-index">3</span>
            <h3>Worker connection</h3>
          </div>
          <p className="launch-active">
            {system.sandbox === "WORKSPACE_SAFE_AVAILABLE"
              ? "Workspace-safe sandboxed worker · bundled · ready"
              : "Workspace-safe sandboxed worker · unavailable"}
          </p>
          <p className="launch-residual">Deferred adapters: {DEFERRED_WORKERS.join(" · ")}</p>
        </li>

        <li className="launch-step">
          <div className="launch-step-head">
            <span className="launch-step-index">4</span>
            <h3>Workspace</h3>
          </div>
          <label className="launch-field">
            <span>Clean local Git repository (absolute path)</span>
            <input
              aria-label="Workspace path"
              value={workspace}
              onChange={(event) => setWorkspace(event.target.value)}
              placeholder="/absolute/path/to/clean/git/repository"
              spellCheck={false}
            />
          </label>
          <p className="launch-residual">Deferred: {DEFERRED_WORKSPACE_SOURCES.join(" · ")}</p>
        </li>

        <li className="launch-step">
          <div className="launch-step-head">
            <span className="launch-step-index">5</span>
            <h3>First mission</h3>
          </div>
          <p>
            Begin opens the mission composer with this workspace. You describe the goal, target file,
            and exact replacement; review authority and budget; then start. Completion is a kernel
            decision — never a model claim.
          </p>
        </li>
      </ol>

      {ready ? null : (
        <div className="launch-refusal" role="alert">
          <strong>BLOCKED_FIRST_LAUNCH</strong>
          <p>These bundled components are not ready, so no mission can start:</p>
          <ul>
            {unready.map(([name, state]) => (
              <li key={name}>
                {name}: {state.replaceAll("_", " ")}
              </li>
            ))}
          </ul>
          <p>Reinstall or repair the bundle. No unready component is treated as ready.</p>
        </div>
      )}

      <button
        type="button"
        className="primary-action"
        onClick={() => onBegin(workspace.trim())}
        disabled={!canBegin}
      >
        Begin first mission
      </button>
      {ready && !workspaceReady ? (
        <p className="launch-hint">Enter the absolute path of a clean Git repository to continue.</p>
      ) : null}
    </section>
  );
}
