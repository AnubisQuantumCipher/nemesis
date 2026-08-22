import { useEffect, useState } from "react";

import {
  normalizeFailure,
  railEntities,
  type CommandFailure,
  type RailEntityView,
} from "../../lib/bridge";
import type { RailPanelProps } from "./props";

const LADDER = [
  "proposed",
  "staged",
  "statically-checked",
  "tested-in-sandbox",
  "evaluated",
  "approved",
  "trusted",
] as const;

export function SkillsPanel({ stateVersion, busy, onBeginMutation }: RailPanelProps) {
  const [entities, setEntities] = useState<RailEntityView[] | null>(null);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [proposeId, setProposeId] = useState("");
  const [proposeName, setProposeName] = useState("");
  const [proposeBody, setProposeBody] = useState("");
  const [approver, setApprover] = useState("");
  const [signature, setSignature] = useState("");

  useEffect(() => {
    let current = true;
    setError(null);
    railEntities("skills")
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

  function advance(entity: RailEntityView) {
    const status = String(entity.value.status ?? "");
    const position = LADDER.indexOf(status as (typeof LADDER)[number]);
    const next = LADDER[position + 1];
    if (!next) {
      return;
    }
    const payload: Record<string, unknown> = { toStatus: next };
    if (next === "approved") {
      payload.approvedBy = approver;
      payload.approvalSignature = signature;
    }
    onBeginMutation({ rail: "skills", verb: "advance", id: entity.id, payload });
  }

  return (
    <section className="rail-panel" aria-labelledby="skills-heading">
      <span className="section-index">RAIL SK / GOVERNED PROCEDURES</span>
      <h2 id="skills-heading">Skills</h2>
      <p className="rail-doctrine">
        Skills are reviewable local procedures. Every lifecycle step is a governed mutation
        through the one-shot authority review; trust states cannot be skipped; the body is
        content-addressed and verified on every read.
      </p>

      {error ? (
        <div className="rail-error" role="alert">
          <strong>{error.code}</strong>
          <span>{error.message}</span>
          <span>{error.recovery}</span>
        </div>
      ) : null}

      {entities === null && !error ? (
        <p className="rail-empty">Loading governed skills…</p>
      ) : null}

      {entities !== null && entities.length === 0 ? (
        <p className="rail-empty">No governed skills. Propose one below.</p>
      ) : null}

      {entities !== null && entities.length > 0 ? (
        <div className="rail-table" role="list" aria-label="Governed skills">
          {entities.map((entity) => {
            const status = String(entity.value.status ?? "UNKNOWN");
            const verified = entity.derived?.bodyVerified === true;
            const revoked = status === "revoked";
            const terminal = status === "trusted" || revoked;
            return (
              <div className="rail-row" role="listitem" key={entity.id}>
                <span className="rail-row-id">{entity.id}</span>
                <span className="rail-row-meta">
                  <span>{String(entity.value.name ?? "")}</span>
                  <span className={revoked ? "rail-tag is-corrupt" : "rail-tag is-armed"}>
                    {status}
                  </span>
                  <span className={verified ? "rail-tag is-verified" : "rail-tag is-corrupt"}>
                    {verified ? "BODY VERIFIED" : "BODY UNVERIFIED"}
                  </span>
                  <span>body {String(entity.value.bodyDigest ?? "").slice(0, 12)}…</span>
                  <span>rev {String(entity.value.revision ?? "")}</span>
                </span>
                <span className="rail-row-actions">
                  {!terminal ? (
                    <button
                      type="button"
                      className="primary-action"
                      disabled={busy}
                      onClick={() => advance(entity)}
                    >
                      Advance
                    </button>
                  ) : null}
                  {!revoked ? (
                    <button
                      type="button"
                      className="danger-action"
                      disabled={busy}
                      onClick={() =>
                        onBeginMutation({
                          rail: "skills",
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
        aria-label="Propose skill"
        onSubmit={(event) => {
          event.preventDefault();
          onBeginMutation({
            rail: "skills",
            verb: "propose",
            id: proposeId,
            payload: { name: proposeName, body: proposeBody },
          });
        }}
      >
        <label>
          Skill id
          <input
            value={proposeId}
            onChange={(event) => setProposeId(event.target.value)}
            placeholder="lower-case-id"
          />
        </label>
        <label>
          Name
          <input
            value={proposeName}
            onChange={(event) => setProposeName(event.target.value)}
            placeholder="Skill name"
          />
        </label>
        <label className="rail-form-span">
          Procedure body
          <textarea
            value={proposeBody}
            onChange={(event) => setProposeBody(event.target.value)}
            placeholder="# Skill procedure…"
          />
        </label>
        <label>
          Approver (for approval step)
          <input
            value={approver}
            onChange={(event) => setApprover(event.target.value)}
            placeholder="operator name"
          />
        </label>
        <label>
          Approval signature (128-hex)
          <input
            value={signature}
            onChange={(event) => setSignature(event.target.value)}
            placeholder="ed25519 signature hex"
          />
        </label>
        <div className="rail-actions rail-form-span">
          <button type="submit" className="primary-action" disabled={busy}>
            Propose skill
          </button>
        </div>
      </form>
    </section>
  );
}
