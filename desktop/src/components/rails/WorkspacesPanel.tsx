import { useEffect, useState } from "react";

import {
  normalizeFailure,
  railEntities,
  type CommandFailure,
  type RailEntityView,
} from "../../lib/bridge";
import type { RailPanelProps } from "./props";

export function WorkspacesPanel({ stateVersion, busy, onBeginMutation }: RailPanelProps) {
  const [entities, setEntities] = useState<RailEntityView[] | null>(null);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [registerId, setRegisterId] = useState("");
  const [registerName, setRegisterName] = useState("");
  const [registerPath, setRegisterPath] = useState("");

  useEffect(() => {
    let current = true;
    setError(null);
    railEntities("workspaces")
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

  return (
    <section className="rail-panel" aria-labelledby="workspaces-heading">
      <span className="section-index">RAIL WS / CANONICAL REPOSITORIES</span>
      <h2 id="workspaces-heading">Workspaces</h2>
      <p className="rail-doctrine">
        Workspaces declare canonical Git repositories missions may target; registration is
        governed; repository health is derived live, never stored.
      </p>

      {error ? (
        <div className="rail-error" role="alert">
          <strong>{error.code}</strong>
          <span>{error.message}</span>
          <span>{error.recovery}</span>
        </div>
      ) : null}

      {entities === null && !error ? (
        <p className="rail-empty">Loading governed workspaces…</p>
      ) : null}

      {entities !== null && entities.length === 0 ? (
        <p className="rail-empty">No governed workspaces. Register one below.</p>
      ) : null}

      {entities !== null && entities.length > 0 ? (
        <div className="rail-table" role="list" aria-label="Governed workspaces">
          {entities.map((entity) => {
            const status = String(entity.value.status ?? "UNKNOWN");
            const revoked = status === "revoked";
            const reachable = entity.derived?.reachable === true;
            const git = entity.derived?.git === true;
            const clean = entity.derived?.clean;
            const head = entity.derived?.head;
            return (
              <div className="rail-row" role="listitem" key={entity.id}>
                <span className="rail-row-id">{entity.id}</span>
                <span className="rail-row-meta">
                  <span>{String(entity.value.name ?? "")}</span>
                  <span>{String(entity.value.canonicalPath ?? "")}</span>
                  <span className={revoked ? "rail-tag is-corrupt" : "rail-tag is-armed"}>
                    {status}
                  </span>
                  <span
                    className={
                      reachable && git ? "rail-tag is-verified" : "rail-tag is-corrupt"
                    }
                  >
                    {reachable && git ? "GIT OK" : !reachable ? "UNREACHABLE" : "NOT GIT"}
                  </span>
                  {clean === true ? (
                    <span className="rail-tag is-verified">CLEAN</span>
                  ) : null}
                  {clean === false ? <span className="rail-tag is-armed">DIRTY</span> : null}
                  {typeof head === "string" && head.length > 0 ? (
                    <span>head {head.slice(0, 12)}</span>
                  ) : null}
                  <span>rev {String(entity.value.revision ?? "")}</span>
                </span>
                <span className="rail-row-actions">
                  {!revoked ? (
                    <button
                      type="button"
                      className="danger-action"
                      disabled={busy}
                      onClick={() =>
                        onBeginMutation({
                          rail: "workspaces",
                          verb: "revoke",
                          id: entity.id,
                          payload: {},
                        })
                      }
                    >
                      Revoke
                    </button>
                  ) : null}
                </span>
              </div>
            );
          })}
        </div>
      ) : null}

      <form
        className="rail-form"
        aria-label="Register workspace"
        onSubmit={(event) => {
          event.preventDefault();
          onBeginMutation({
            rail: "workspaces",
            verb: "register",
            id: registerId,
            payload: { name: registerName, canonicalPath: registerPath },
          });
        }}
      >
        <label>
          Workspace id
          <input
            value={registerId}
            onChange={(event) => setRegisterId(event.target.value)}
            placeholder="lower-case-id"
          />
        </label>
        <label>
          Name
          <input
            value={registerName}
            onChange={(event) => setRegisterName(event.target.value)}
            placeholder="Workspace name"
          />
        </label>
        <label className="rail-form-span">
          Canonical path
          <input
            value={registerPath}
            onChange={(event) => setRegisterPath(event.target.value)}
            placeholder="/absolute/path/to/repo"
          />
        </label>
        <div className="rail-actions rail-form-span">
          <button type="submit" className="primary-action" disabled={busy}>
            Register workspace
          </button>
        </div>
      </form>
    </section>
  );
}
