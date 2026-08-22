import { useEffect, useState } from "react";

import {
  dispatchAutomation,
  normalizeFailure,
  railEntities,
  type CommandFailure,
  type RailEntityView,
} from "../../lib/bridge";
import type { RailPanelProps } from "./props";

interface DispatchOutcome {
  automationId: string;
  verdict: string;
  boundary: string | null;
}

function truncateGoal(goal: string): string {
  return goal.length > 48 ? `${goal.slice(0, 48)}…` : goal;
}

export function AutomationsPanel({ stateVersion, busy, onBeginMutation }: RailPanelProps) {
  const [entities, setEntities] = useState<RailEntityView[] | null>(null);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [dispatchingId, setDispatchingId] = useState<string | null>(null);
  const [lastDispatch, setLastDispatch] = useState<DispatchOutcome | null>(null);
  const [enqueueId, setEnqueueId] = useState("");
  const [enqueueName, setEnqueueName] = useState("");
  const [triggerKind, setTriggerKind] = useState("manual");
  const [triggerKey, setTriggerKey] = useState("");
  const [dueUnixSeconds, setDueUnixSeconds] = useState("");
  const [goal, setGoal] = useState("");
  const [workspaceId, setWorkspaceId] = useState("");
  const [relativePath, setRelativePath] = useState("");
  const [replacement, setReplacement] = useState("");

  useEffect(() => {
    let current = true;
    setError(null);
    railEntities("automations")
      .then((response) => {
        if (current) {
          setEntities(response.entities);
        }
      })
      .catch((cause) => {
        if (current) {
          setEntities(null);
          setError(normalizeFailure(cause));
        }
      });
    return () => {
      current = false;
    };
  }, [stateVersion]);

  async function dispatch(entity: RailEntityView) {
    setDispatchingId(entity.id);
    setLastDispatch(null);
    try {
      const receipt = await dispatchAutomation(entity.id);
      const boundary = receipt.detail?.boundary;
      setLastDispatch({
        automationId: entity.id,
        verdict: receipt.verdict,
        boundary: typeof boundary === "string" ? boundary : null,
      });
    } catch (cause) {
      setError(normalizeFailure(cause));
    } finally {
      setDispatchingId(null);
    }
  }

  function enqueue() {
    const trigger: Record<string, unknown> = { kind: triggerKind, key: triggerKey };
    if (triggerKind === "local-schedule") {
      trigger.dueUnixSeconds = Number(dueUnixSeconds);
    }
    onBeginMutation({
      rail: "automations",
      verb: "enqueue",
      id: enqueueId,
      payload: {
        name: enqueueName,
        operation: "draft-mission",
        trigger,
        missionTemplate: { goal, workspaceId, relativePath, replacement },
      },
    });
  }

  return (
    <section className="rail-panel" aria-labelledby="automations-heading">
      <span className="section-index">RAIL AU / TRIGGERS STOP AT AUTHORITY</span>
      <h2 id="automations-heading">Automations</h2>
      <p className="rail-doctrine">
        Automations may only draft missions for human review; compile, review, approval, and
        run remain human acts; every dispatch decision is receipted, including refusals.
      </p>

      {error ? (
        <div className="rail-error" role="alert">
          <strong>{error.code}</strong>
          <span>{error.message}</span>
          <span>{error.recovery}</span>
        </div>
      ) : null}

      {entities === null && !error ? (
        <p className="rail-empty">Loading governed automations…</p>
      ) : null}

      {entities !== null && entities.length === 0 ? (
        <p className="rail-empty">No governed automations. Enqueue one below.</p>
      ) : null}

      {entities !== null && entities.length > 0 ? (
        <div className="rail-table" role="list" aria-label="Governed automations">
          {entities.map((entity) => {
            const status = String(entity.value.status ?? "UNKNOWN");
            const queued = status === "queued";
            const trigger = (entity.value.trigger ?? {}) as Record<string, unknown>;
            const template = (entity.value.missionTemplate ?? {}) as Record<string, unknown>;
            const due = Number(trigger.dueUnixSeconds ?? 0);
            const outcome =
              lastDispatch && lastDispatch.automationId === entity.id ? lastDispatch : null;
            return (
              <div className="rail-row" role="listitem" key={entity.id}>
                <span className="rail-row-id">{entity.id}</span>
                <span className="rail-row-meta">
                  <span>{String(entity.value.name ?? "")}</span>
                  <span>
                    {String(trigger.kind ?? "")} {String(trigger.key ?? "")}
                  </span>
                  {due > 0 ? <span>due {due}</span> : null}
                  <span>{truncateGoal(String(template.goal ?? ""))}</span>
                  <span>{String(template.workspaceId ?? "")}</span>
                  <span className={queued ? "rail-tag is-armed" : "rail-tag is-corrupt"}>
                    {status}
                  </span>
                  <span>rev {String(entity.value.revision ?? "")}</span>
                  {outcome ? (
                    <span
                      className={
                        outcome.verdict === "AUTHORIZED_DRAFT_ONLY"
                          ? "rail-tag is-verified"
                          : "rail-tag is-armed"
                      }
                    >
                      {outcome.verdict}
                    </span>
                  ) : null}
                  {outcome?.boundary ? <span>{outcome.boundary}</span> : null}
                </span>
                <span className="rail-row-actions">
                  {queued ? (
                    <button
                      type="button"
                      className="primary-action"
                      disabled={busy || dispatchingId !== null}
                      onClick={() => dispatch(entity)}
                    >
                      Dispatch
                    </button>
                  ) : null}
                  <button
                    type="button"
                    className="danger-action"
                    disabled={busy}
                    onClick={() =>
                      onBeginMutation({
                        rail: "automations",
                        verb: "revoke",
                        id: entity.id,
                        payload: {},
                      })
                    }
                  >
                    Revoke
                  </button>
                </span>
              </div>
            );
          })}
        </div>
      ) : null}

      <form
        className="rail-form"
        aria-label="Enqueue automation"
        onSubmit={(event) => {
          event.preventDefault();
          enqueue();
        }}
      >
        <label>
          Automation id
          <input
            value={enqueueId}
            onChange={(event) => setEnqueueId(event.target.value)}
            placeholder="lower-case-id"
          />
        </label>
        <label>
          Name
          <input
            value={enqueueName}
            onChange={(event) => setEnqueueName(event.target.value)}
            placeholder="Automation name"
          />
        </label>
        <label>
          Trigger kind
          <select value={triggerKind} onChange={(event) => setTriggerKind(event.target.value)}>
            <option value="manual">manual</option>
            <option value="local-schedule">local-schedule</option>
          </select>
        </label>
        <label>
          Trigger key
          <input
            value={triggerKey}
            onChange={(event) => setTriggerKey(event.target.value)}
            placeholder="trigger key"
          />
        </label>
        <label>
          Due unix seconds
          <input
            type="number"
            value={dueUnixSeconds}
            onChange={(event) => setDueUnixSeconds(event.target.value)}
            placeholder="0"
          />
        </label>
        <label>
          Goal
          <input
            value={goal}
            onChange={(event) => setGoal(event.target.value)}
            placeholder="Mission goal"
          />
        </label>
        <label>
          Workspace id
          <input
            value={workspaceId}
            onChange={(event) => setWorkspaceId(event.target.value)}
            placeholder="workspace id"
          />
        </label>
        <label>
          Relative path
          <input
            value={relativePath}
            onChange={(event) => setRelativePath(event.target.value)}
            placeholder="path/in/workspace"
          />
        </label>
        <label className="rail-form-span">
          Replacement
          <textarea
            value={replacement}
            onChange={(event) => setReplacement(event.target.value)}
            placeholder="replacement content"
          />
        </label>
        <div className="rail-actions rail-form-span">
          <button type="submit" className="primary-action" disabled={busy}>
            Enqueue automation
          </button>
        </div>
      </form>
    </section>
  );
}
