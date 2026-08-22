import { useEffect, useState } from "react";

import {
  normalizeFailure,
  railEntities,
  type CommandFailure,
  type RailEntityView,
} from "../../lib/bridge";
import type { RailPanelProps } from "./props";

const PROVIDER_KINDS = [
  "generic-subprocess",
  "codex-cli",
  "claude-code-cli",
  "local-model",
  "openai-api",
  "anthropic-api",
] as const;

const ROLES = [
  "planner",
  "builder",
  "reviewer",
  "red-team",
  "verifier",
  "integrator",
  "recovery",
] as const;

const CEILINGS = [
  "informational",
  "advisory",
  "decision-boundary",
  "safety-critical",
] as const;

const AVAILABILITY_TAG: Record<string, string> = {
  READY: "rail-tag is-verified",
  MISSING_EXECUTABLE: "rail-tag is-corrupt",
  UNAVAILABLE_NETWORK_DENIED: "rail-tag is-armed",
};

export function AgentsPanel({ stateVersion, busy, onBeginMutation }: RailPanelProps) {
  const [entities, setEntities] = useState<RailEntityView[] | null>(null);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [registerId, setRegisterId] = useState("");
  const [registerName, setRegisterName] = useState("");
  const [providerKind, setProviderKind] = useState<string>(PROVIDER_KINDS[0]);
  const [executablePath, setExecutablePath] = useState("");
  const [roles, setRoles] = useState<string[]>([]);
  const [consequenceCeiling, setConsequenceCeiling] = useState<string>(CEILINGS[0]);
  const [costMicrounits, setCostMicrounits] = useState("0");
  const [inputTokens, setInputTokens] = useState("0");
  const [outputTokens, setOutputTokens] = useState("0");

  useEffect(() => {
    let current = true;
    setError(null);
    railEntities("agents")
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

  function toggleRole(role: string, checked: boolean) {
    setRoles((previous) => {
      const without = previous.filter((entry) => entry !== role);
      return checked ? [...without, role] : without;
    });
  }

  function register() {
    const payload: Record<string, unknown> = {
      name: registerName,
      providerKind,
      roles: ROLES.filter((role) => roles.includes(role)),
      consequenceCeiling,
      budget: {
        costMicrounits: Number(costMicrounits),
        inputTokens: Number(inputTokens),
        outputTokens: Number(outputTokens),
      },
    };
    if (executablePath !== "") {
      payload.executablePath = executablePath;
    }
    onBeginMutation({ rail: "agents", verb: "register", id: registerId, payload });
  }

  return (
    <section className="rail-panel" aria-labelledby="agents-heading">
      <span className="section-index">RAIL AG / UNTRUSTED WORKERS</span>
      <h2 id="agents-heading">Agents</h2>
      <p className="rail-doctrine">
        Agents propose, the kernel decides; every agent carries declared roles, a consequence
        ceiling, and a bounded usage budget; API providers are permanently unavailable while
        network authority is denied.
      </p>

      {error ? (
        <div className="rail-error" role="alert">
          <strong>{error.code}</strong>
          <span>{error.message}</span>
          <span>{error.recovery}</span>
        </div>
      ) : null}

      {entities === null && !error ? (
        <p className="rail-empty">Loading governed agents…</p>
      ) : null}

      {entities !== null && entities.length === 0 ? (
        <p className="rail-empty">No governed agents. Register one below.</p>
      ) : null}

      {entities !== null && entities.length > 0 ? (
        <div className="rail-table" role="list" aria-label="Governed agents">
          {entities.map((entity) => {
            const status = String(entity.value.status ?? "UNKNOWN");
            const revoked = status === "revoked";
            const availability = String(entity.derived?.availability ?? "UNKNOWN");
            return (
              <div className="rail-row" role="listitem" key={entity.id}>
                <span className="rail-row-id">{entity.id}</span>
                <span className="rail-row-meta">
                  <span>{String(entity.value.name ?? "")}</span>
                  <span>{String(entity.value.providerKind ?? "")}</span>
                  <span>
                    {Array.isArray(entity.value.roles)
                      ? (entity.value.roles as string[]).join(", ")
                      : ""}
                  </span>
                  <span>{String(entity.value.consequenceCeiling ?? "")}</span>
                  <span className={AVAILABILITY_TAG[availability] ?? "rail-tag is-corrupt"}>
                    {availability}
                  </span>
                  <span className={revoked ? "rail-tag is-corrupt" : "rail-tag is-armed"}>
                    {status}
                  </span>
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
                          rail: "agents",
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
        aria-label="Register agent"
        onSubmit={(event) => {
          event.preventDefault();
          register();
        }}
      >
        <label>
          Agent id
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
            placeholder="Agent name"
          />
        </label>
        <label>
          Provider kind
          <select
            value={providerKind}
            onChange={(event) => setProviderKind(event.target.value)}
          >
            {PROVIDER_KINDS.map((kind) => (
              <option key={kind} value={kind}>
                {kind}
              </option>
            ))}
          </select>
        </label>
        <label>
          Executable path
          <input
            value={executablePath}
            onChange={(event) => setExecutablePath(event.target.value)}
            placeholder="/usr/local/bin/agent (subprocess kinds only)"
          />
        </label>
        <fieldset className="rail-form-span">
          <legend>Roles</legend>
          {ROLES.map((role) => (
            <label key={role}>
              {role}
              <input
                type="checkbox"
                checked={roles.includes(role)}
                onChange={(event) => toggleRole(role, event.target.checked)}
              />
            </label>
          ))}
        </fieldset>
        <label>
          Consequence ceiling
          <select
            value={consequenceCeiling}
            onChange={(event) => setConsequenceCeiling(event.target.value)}
          >
            {CEILINGS.map((ceiling) => (
              <option key={ceiling} value={ceiling}>
                {ceiling}
              </option>
            ))}
          </select>
        </label>
        <label>
          Budget cost microunits
          <input
            type="number"
            value={costMicrounits}
            onChange={(event) => setCostMicrounits(event.target.value)}
          />
        </label>
        <label>
          Budget input tokens
          <input
            type="number"
            value={inputTokens}
            onChange={(event) => setInputTokens(event.target.value)}
          />
        </label>
        <label>
          Budget output tokens
          <input
            type="number"
            value={outputTokens}
            onChange={(event) => setOutputTokens(event.target.value)}
          />
        </label>
        <div className="rail-actions rail-form-span">
          <button type="submit" className="primary-action" disabled={busy}>
            Register agent
          </button>
        </div>
      </form>
    </section>
  );
}
