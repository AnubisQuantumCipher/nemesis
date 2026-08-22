import { useEffect, useState } from "react";

import {
  executeIntegrationPlugin,
  normalizeFailure,
  railEntities,
  type ChainedReceipt,
  type CommandFailure,
  type RailEntityView,
} from "../../lib/bridge";
import type { RailPanelProps } from "./props";

const PROVIDER_KINDS = [
  "codex-cli",
  "claude-code-cli",
  "local-model",
  "openai-api",
  "anthropic-api",
] as const;

interface ToolRef {
  name: string;
  schemaDigest: string;
}

function statusTagClass(status: string): string {
  if (status === "enabled") {
    return "rail-tag is-verified";
  }
  if (status === "revoked") {
    return "rail-tag is-corrupt";
  }
  return "rail-tag is-armed";
}

export function IntegrationsPanel({ stateVersion, busy, onBeginMutation }: RailPanelProps) {
  const [entities, setEntities] = useState<RailEntityView[] | null>(null);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [receipts, setReceipts] = useState<Record<string, ChainedReceipt>>({});
  const [executingId, setExecutingId] = useState<string | null>(null);
  const [formId, setFormId] = useState("");
  const [formName, setFormName] = useState("");
  const [formKind, setFormKind] = useState("wasm-plugin");
  const [formProviderKind, setFormProviderKind] = useState<string>(PROVIDER_KINDS[0]);
  const [formModuleWat, setFormModuleWat] = useState("");
  const [formExport, setFormExport] = useState("");
  const [formToolName, setFormToolName] = useState("");
  const [formToolDigest, setFormToolDigest] = useState("");

  useEffect(() => {
    let current = true;
    setError(null);
    railEntities("integrations")
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

  async function execute(entityId: string) {
    setExecutingId(entityId);
    try {
      const receipt = await executeIntegrationPlugin(entityId);
      setReceipts((previous) => ({ ...previous, [entityId]: receipt }));
    } catch (cause) {
      setError(normalizeFailure(cause));
    } finally {
      setExecutingId(null);
    }
  }

  function register() {
    const tools: ToolRef[] = [{ name: formToolName, schemaDigest: formToolDigest }];
    const payload: Record<string, unknown> = {
      name: formName,
      kind: formKind,
      tools,
    };
    if (formKind === "wasm-plugin") {
      payload.moduleWat = formModuleWat;
      payload.export = formExport;
    } else {
      payload.providerKind = formProviderKind;
    }
    onBeginMutation({ rail: "integrations", verb: "register", id: formId, payload });
  }

  return (
    <section className="rail-panel" aria-labelledby="integrations-heading">
      <span className="section-index">RAIL IN / PROVIDERS PROPOSE, NEMESIS GOVERNS</span>
      <h2 id="integrations-heading">Integrations</h2>
      <p className="rail-doctrine">
        Integrations expose tools behind frozen schema digests; capabilities are
        deny-by-default and unrepresentable beyond empty; wasm plugins run in a bounded
        WASI host with no network, filesystem, secrets, or events.
      </p>

      {error ? (
        <div className="rail-error" role="alert">
          <strong>{error.code}</strong>
          <span>{error.message}</span>
          <span>{error.recovery}</span>
        </div>
      ) : null}

      {entities === null && !error ? (
        <p className="rail-empty">Loading governed integrations…</p>
      ) : null}

      {entities !== null && entities.length === 0 ? (
        <p className="rail-empty">No governed integrations. Register one below.</p>
      ) : null}

      {entities !== null && entities.length > 0 ? (
        <div className="rail-table" role="list" aria-label="Governed integrations">
          {entities.map((entity) => {
            const status = String(entity.value.status ?? "UNKNOWN");
            const kind = String(entity.value.kind ?? "");
            const providerKind = entity.value.providerKind;
            const moduleDigest = entity.value.moduleDigest;
            const tools = Array.isArray(entity.value.tools)
              ? (entity.value.tools as Array<Record<string, unknown>>)
              : [];
            const receipt = receipts[entity.id];
            const revoked = status === "revoked";
            return (
              <div className="rail-row" role="listitem" key={entity.id}>
                <span className="rail-row-id">{entity.id}</span>
                <span className="rail-row-meta">
                  <span>{String(entity.value.name ?? "")}</span>
                  <span>{kind}</span>
                  {typeof providerKind === "string" ? <span>{providerKind}</span> : null}
                  {tools.map((tool) => (
                    <span key={String(tool.name ?? "")}>
                      tool {String(tool.name ?? "")}{" "}
                      {String(tool.schemaDigest ?? "").slice(0, 8)}
                    </span>
                  ))}
                  {typeof moduleDigest === "string" ? (
                    <span>module {moduleDigest.slice(0, 12)}…</span>
                  ) : null}
                  <span className="rail-tag is-verified">
                    NET DENY / FS DENY / SECRETS DENY
                  </span>
                  <span className={statusTagClass(status)}>{status}</span>
                  <span>rev {String(entity.value.revision ?? "")}</span>
                  {receipt ? (
                    <>
                      <span
                        className={
                          receipt.verdict === "EXECUTED"
                            ? "rail-tag is-verified"
                            : "rail-tag is-corrupt"
                        }
                      >
                        {receipt.verdict}
                      </span>
                      {receipt.detail && receipt.detail.result !== undefined ? (
                        <span>{String(receipt.detail.result)}</span>
                      ) : null}
                    </>
                  ) : null}
                </span>
                <span className="rail-row-actions">
                  {status === "proposed" || status === "disabled" ? (
                    <button
                      type="button"
                      className="primary-action"
                      disabled={busy}
                      onClick={() =>
                        onBeginMutation({
                          rail: "integrations",
                          verb: "enable",
                          id: entity.id,
                          payload: {},
                        })
                      }
                    >
                      Enable
                    </button>
                  ) : null}
                  {status === "enabled" ? (
                    <button
                      type="button"
                      className="primary-action"
                      disabled={busy}
                      onClick={() =>
                        onBeginMutation({
                          rail: "integrations",
                          verb: "disable",
                          id: entity.id,
                          payload: {},
                        })
                      }
                    >
                      Disable
                    </button>
                  ) : null}
                  {status === "enabled" && kind === "wasm-plugin" ? (
                    <button
                      type="button"
                      className="primary-action"
                      disabled={executingId !== null}
                      onClick={() => {
                        void execute(entity.id);
                      }}
                    >
                      Execute
                    </button>
                  ) : null}
                  {!revoked ? (
                    <button
                      type="button"
                      className="danger-action"
                      disabled={busy}
                      onClick={() =>
                        onBeginMutation({
                          rail: "integrations",
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
        aria-label="Register integration"
        onSubmit={(event) => {
          event.preventDefault();
          register();
        }}
      >
        <label>
          Integration id
          <input
            value={formId}
            onChange={(event) => setFormId(event.target.value)}
            placeholder="lower-case-id"
          />
        </label>
        <label>
          Name
          <input
            value={formName}
            onChange={(event) => setFormName(event.target.value)}
            placeholder="Integration name"
          />
        </label>
        <label>
          Kind
          <select value={formKind} onChange={(event) => setFormKind(event.target.value)}>
            <option value="wasm-plugin">wasm-plugin</option>
            <option value="external-provider">external-provider</option>
          </select>
        </label>
        {formKind === "external-provider" ? (
          <label>
            Provider kind
            <select
              value={formProviderKind}
              onChange={(event) => setFormProviderKind(event.target.value)}
            >
              {PROVIDER_KINDS.map((providerKind) => (
                <option key={providerKind} value={providerKind}>
                  {providerKind}
                </option>
              ))}
            </select>
          </label>
        ) : null}
        {formKind === "wasm-plugin" ? (
          <label className="rail-form-span">
            Module WAT
            <textarea
              value={formModuleWat}
              onChange={(event) => setFormModuleWat(event.target.value)}
              placeholder="(module …)"
            />
          </label>
        ) : null}
        {formKind === "wasm-plugin" ? (
          <label>
            Export
            <input
              value={formExport}
              onChange={(event) => setFormExport(event.target.value)}
              placeholder="exported function"
            />
          </label>
        ) : null}
        <label>
          Tool name
          <input
            value={formToolName}
            onChange={(event) => setFormToolName(event.target.value)}
            placeholder="tool name"
          />
        </label>
        <label>
          Tool schema digest (64-hex)
          <input
            value={formToolDigest}
            onChange={(event) => setFormToolDigest(event.target.value)}
            placeholder="sha-256 of the frozen tool schema"
          />
        </label>
        <div className="rail-actions rail-form-span">
          <button type="submit" className="primary-action" disabled={busy}>
            Register integration
          </button>
        </div>
      </form>
    </section>
  );
}
