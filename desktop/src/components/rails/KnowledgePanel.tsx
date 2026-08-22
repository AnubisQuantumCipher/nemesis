import { useEffect, useState } from "react";

import {
  normalizeFailure,
  railEntities,
  type CommandFailure,
  type RailEntityView,
} from "../../lib/bridge";
import type { RailPanelProps } from "./props";

const SCOPES = ["mission", "workspace", "global"] as const;
const ACTIVE_STATUSES = ["proposed", "observed", "verified", "disputed"] as const;

interface KnowledgeContradiction {
  subject: string;
  predicate: string;
  entries: [string, string];
}

interface RowTransition {
  toStatus: string;
  contradictedBy: string;
}

function transitionTargets(status: string): string[] {
  const targets: string[] = [];
  if (status === "proposed") {
    targets.push("observed");
  }
  if (status === "observed") {
    targets.push("verified");
  }
  if ((ACTIVE_STATUSES as readonly string[]).includes(status) && status !== "disputed") {
    targets.push("disputed");
  }
  return targets;
}

function statusTagClass(status: string): string {
  if (status === "observed" || status === "verified") {
    return "rail-tag is-verified";
  }
  if (status === "disputed" || status === "revoked") {
    return "rail-tag is-corrupt";
  }
  return "rail-tag is-armed";
}

export function KnowledgePanel({ stateVersion, busy, onBeginMutation }: RailPanelProps) {
  const [entities, setEntities] = useState<RailEntityView[] | null>(null);
  const [contradictions, setContradictions] = useState<KnowledgeContradiction[]>([]);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [transitions, setTransitions] = useState<Record<string, RowTransition>>({});
  const [assertId, setAssertId] = useState("");
  const [subject, setSubject] = useState("");
  const [predicate, setPredicate] = useState("");
  const [value, setValue] = useState("");
  const [scope, setScope] = useState<string>("mission");
  const [sourceDigest, setSourceDigest] = useState("");
  const [observedSequence, setObservedSequence] = useState("");
  const [untrustedExternal, setUntrustedExternal] = useState(false);
  const [supersedes, setSupersedes] = useState("");

  useEffect(() => {
    let current = true;
    setError(null);
    railEntities("knowledge")
      .then((response) => {
        if (current) {
          setEntities(response.entities);
          setContradictions((response.contradictions ?? []) as unknown as KnowledgeContradiction[]);
        }
      })
      .catch((cause) => {
        if (current) {
          setEntities(null);
          setContradictions([]);
          setError(normalizeFailure(cause));
        }
      });
    return () => {
      current = false;
    };
  }, [stateVersion]);

  function applyTransition(entity: RailEntityView, row: RowTransition) {
    if (!row.toStatus) {
      return;
    }
    const payload: Record<string, unknown> = { toStatus: row.toStatus };
    if (row.toStatus === "disputed") {
      payload.contradictedBy = row.contradictedBy;
    }
    onBeginMutation({ rail: "knowledge", verb: "transition", id: entity.id, payload });
  }

  function submitAssert() {
    const payload: Record<string, unknown> = {
      subject,
      predicate,
      value,
      scope,
      sourceDigest,
      observedSequence: Number(observedSequence),
      untrustedExternal,
    };
    if (supersedes.trim() !== "") {
      payload.supersedes = supersedes;
    }
    onBeginMutation({ rail: "knowledge", verb: "assert", id: assertId, payload });
  }

  return (
    <section className="rail-panel" aria-labelledby="knowledge-heading">
      <span className="section-index">RAIL KN / PROVENANCE-AWARE FACTS</span>
      <h2 id="knowledge-heading">Knowledge</h2>
      <p className="rail-doctrine">
        Only observed and verified entries are active truth; contradictions are surfaced,
        never resolved silently; supersession and revocation preserve history.
      </p>

      {error ? (
        <div className="rail-error" role="alert">
          <strong>{error.code}</strong>
          <span>{error.message}</span>
          <span>{error.recovery}</span>
        </div>
      ) : null}

      {contradictions.length > 0 ? (
        <div className="rail-error" role="alert">
          {contradictions.map((contradiction) => (
            <span key={`${contradiction.subject}-${contradiction.predicate}`}>
              CONTRADICTION {contradiction.subject} {contradiction.predicate}:{" "}
              {contradiction.entries[0]} vs {contradiction.entries[1]}
            </span>
          ))}
        </div>
      ) : null}

      {entities === null && !error ? (
        <p className="rail-empty">Loading knowledge entries…</p>
      ) : null}

      {entities !== null && entities.length === 0 ? (
        <p className="rail-empty">No knowledge entries. Assert one below.</p>
      ) : null}

      {entities !== null && entities.length > 0 ? (
        <div className="rail-table" role="list" aria-label="Knowledge entries">
          {entities.map((entity) => {
            const status = String(entity.value.status ?? "UNKNOWN");
            const targets = transitionTargets(status);
            const row = transitions[entity.id] ?? {
              toStatus: targets[0] ?? "",
              contradictedBy: "",
            };
            const revoked = status === "revoked";
            return (
              <div className="rail-row" role="listitem" key={entity.id}>
                <span className="rail-row-id">{entity.id}</span>
                <span className="rail-row-meta">
                  <span>{String(entity.value.subject ?? "")}</span>
                  <span>{String(entity.value.predicate ?? "")}</span>
                  <span>
                    {String(entity.value.value ?? "").length > 48
                      ? `${String(entity.value.value ?? "").slice(0, 48)}…`
                      : String(entity.value.value ?? "")}
                  </span>
                  <span>{String(entity.value.scope ?? "")}</span>
                  <span className={statusTagClass(status)}>{status}</span>
                  {entity.value.untrustedExternal === true ? (
                    <span className="rail-tag is-armed">EXTERNAL UNTRUSTED</span>
                  ) : null}
                  <span>rev {String(entity.value.revision ?? "")}</span>
                </span>
                <span className="rail-row-actions">
                  {targets.length > 0 ? (
                    <>
                      <select
                        aria-label={`Transition ${entity.id}`}
                        value={row.toStatus}
                        onChange={(event) =>
                          setTransitions((previous) => ({
                            ...previous,
                            [entity.id]: { ...row, toStatus: event.target.value },
                          }))
                        }
                      >
                        {targets.map((target) => (
                          <option key={target} value={target}>
                            {target}
                          </option>
                        ))}
                      </select>
                      {row.toStatus === "disputed" ? (
                        <input
                          aria-label={`Contradicted by ${entity.id}`}
                          value={row.contradictedBy}
                          onChange={(event) =>
                            setTransitions((previous) => ({
                              ...previous,
                              [entity.id]: { ...row, contradictedBy: event.target.value },
                            }))
                          }
                          placeholder="contradicting entry id"
                        />
                      ) : null}
                      <button
                        type="button"
                        className="primary-action"
                        disabled={busy}
                        onClick={() => applyTransition(entity, row)}
                      >
                        Apply
                      </button>
                    </>
                  ) : null}
                  {!revoked ? (
                    <button
                      type="button"
                      className="danger-action"
                      disabled={busy}
                      onClick={() =>
                        onBeginMutation({
                          rail: "knowledge",
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
        aria-label="Assert knowledge entry"
        onSubmit={(event) => {
          event.preventDefault();
          submitAssert();
        }}
      >
        <label>
          Entry id
          <input
            value={assertId}
            onChange={(event) => setAssertId(event.target.value)}
            placeholder="lower-case-id"
          />
        </label>
        <label>
          Subject
          <input
            value={subject}
            onChange={(event) => setSubject(event.target.value)}
            placeholder="subject"
          />
        </label>
        <label>
          Predicate
          <input
            value={predicate}
            onChange={(event) => setPredicate(event.target.value)}
            placeholder="predicate"
          />
        </label>
        <label className="rail-form-span">
          Value
          <input
            value={value}
            onChange={(event) => setValue(event.target.value)}
            placeholder="observed value"
          />
        </label>
        <label>
          Scope
          <select value={scope} onChange={(event) => setScope(event.target.value)}>
            {SCOPES.map((option) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
        </label>
        <label>
          Source digest (64-hex)
          <input
            value={sourceDigest}
            onChange={(event) => setSourceDigest(event.target.value)}
            placeholder="sha-256 of the source"
          />
        </label>
        <label>
          Observed sequence
          <input
            type="number"
            value={observedSequence}
            onChange={(event) => setObservedSequence(event.target.value)}
            placeholder="0"
          />
        </label>
        <label>
          Untrusted external
          <input
            type="checkbox"
            checked={untrustedExternal}
            onChange={(event) => setUntrustedExternal(event.target.checked)}
          />
        </label>
        <label>
          Supersedes (optional)
          <input
            value={supersedes}
            onChange={(event) => setSupersedes(event.target.value)}
            placeholder="superseded entry id"
          />
        </label>
        <div className="rail-actions rail-form-span">
          <button type="submit" className="primary-action" disabled={busy}>
            Assert entry
          </button>
        </div>
      </form>
    </section>
  );
}
