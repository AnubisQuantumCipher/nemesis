import { useEffect, useState } from "react";

import {
  normalizeFailure,
  railEntities,
  runRailTest,
  type CommandFailure,
  type RailEntityView,
} from "../../lib/bridge";
import type { RailPanelProps } from "./props";

type TestRun = Record<string, unknown>;

export function TestsPanel({ stateVersion, busy, onBeginMutation }: RailPanelProps) {
  const [entities, setEntities] = useState<RailEntityView[] | null>(null);
  const [latestRuns, setLatestRuns] = useState<TestRun[]>([]);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [lastReceipt, setLastReceipt] = useState<{ testId: string; verdict: string } | null>(
    null,
  );
  const [runningId, setRunningId] = useState<string | null>(null);
  const [registerId, setRegisterId] = useState("");
  const [registerName, setRegisterName] = useState("");
  const [registerKind, setRegisterKind] = useState("git-diff-check");
  const [registerWorkspaceId, setRegisterWorkspaceId] = useState("");
  const [registerRelativePath, setRegisterRelativePath] = useState("");
  const [registerExpectedSha, setRegisterExpectedSha] = useState("");

  useEffect(() => {
    let current = true;
    setError(null);
    railEntities("tests")
      .then((response) => {
        if (current) {
          setEntities(response.entities);
          setLatestRuns(response.latestRuns ?? []);
        }
      })
      .catch((cause) => {
        if (current) {
          setEntities(null);
          setLatestRuns([]);
          setError(normalizeFailure(cause));
        }
      });
    return () => {
      current = false;
    };
  }, [stateVersion]);

  async function run(entity: RailEntityView) {
    setError(null);
    setRunningId(entity.id);
    try {
      const receipt = await runRailTest(entity.id);
      setLastReceipt({ testId: entity.id, verdict: receipt.verdict });
      const response = await railEntities("tests");
      setEntities(response.entities);
      setLatestRuns(response.latestRuns ?? []);
    } catch (cause) {
      setError(normalizeFailure(cause));
    } finally {
      setRunningId(null);
    }
  }

  function register(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const payload: Record<string, unknown> = {
      name: registerName,
      kind: registerKind,
      workspaceId: registerWorkspaceId,
      relativePath: registerRelativePath,
    };
    if (registerKind === "content-match") {
      payload.expectedSha256 = registerExpectedSha;
    }
    onBeginMutation({ rail: "tests", verb: "register", id: registerId, payload });
  }

  return (
    <section className="rail-panel" aria-labelledby="tests-heading">
      <span className="section-index">RAIL TS / DETERMINISTIC CHECKS</span>
      <h2 id="tests-heading">Tests</h2>
      <p className="rail-doctrine">
        Tests bind deterministic checks to exact source; a run records the observed source
        digest in a hash-chained receipt; runs against changed source surface as STALE,
        never silently green.
      </p>

      {error ? (
        <div className="rail-error" role="alert">
          <strong>{error.code}</strong>
          <span>{error.message}</span>
          <span>{error.recovery}</span>
        </div>
      ) : null}

      {entities === null && !error ? (
        <p className="rail-empty">Loading governed tests…</p>
      ) : null}

      {entities !== null && entities.length === 0 ? (
        <p className="rail-empty">No governed tests. Register one below.</p>
      ) : null}

      {entities !== null && entities.length > 0 ? (
        <div className="rail-table" role="list" aria-label="Governed tests">
          {entities.map((entity) => {
            const status = String(entity.value.status ?? "UNKNOWN");
            const revoked = status === "revoked";
            const active = status === "active";
            const latest = latestRuns.find((entry) => entry.testId === entity.id);
            const verdict = latest ? String(latest.verdict ?? "") : null;
            const stale = latest?.stale === true;
            return (
              <div className="rail-row" role="listitem" key={entity.id}>
                <span className="rail-row-id">{entity.id}</span>
                <span className="rail-row-meta">
                  <span>{String(entity.value.name ?? "")}</span>
                  <span>{String(entity.value.kind ?? "")}</span>
                  <span>{String(entity.value.workspaceId ?? "")}</span>
                  <span>{String(entity.value.relativePath ?? "")}</span>
                  <span className={revoked ? "rail-tag is-corrupt" : "rail-tag is-armed"}>
                    {status}
                  </span>
                  {latest ? (
                    <span
                      className={
                        verdict === "PASS" ? "rail-tag is-verified" : "rail-tag is-corrupt"
                      }
                    >
                      {verdict}
                    </span>
                  ) : null}
                  {latest && stale ? <span className="rail-tag is-armed">STALE</span> : null}
                  {latest ? (
                    <span>src {String(latest.sourceDigest ?? "").slice(0, 12)}…</span>
                  ) : null}
                  {lastReceipt && lastReceipt.testId === entity.id ? (
                    <span>RECEIPT {lastReceipt.verdict}</span>
                  ) : null}
                </span>
                <span className="rail-row-actions">
                  {active ? (
                    <button
                      type="button"
                      className="primary-action"
                      disabled={runningId !== null}
                      onClick={() => {
                        void run(entity);
                      }}
                    >
                      Run
                    </button>
                  ) : null}
                  {!revoked ? (
                    <button
                      type="button"
                      className="danger-action"
                      disabled={busy}
                      onClick={() =>
                        onBeginMutation({
                          rail: "tests",
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

      <form className="rail-form" aria-label="Register test" onSubmit={register}>
        <label>
          Test id
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
            placeholder="Test name"
          />
        </label>
        <label>
          Kind
          <select
            value={registerKind}
            onChange={(event) => setRegisterKind(event.target.value)}
          >
            <option value="git-diff-check">git-diff-check</option>
            <option value="content-match">content-match</option>
          </select>
        </label>
        <label>
          Workspace id
          <input
            value={registerWorkspaceId}
            onChange={(event) => setRegisterWorkspaceId(event.target.value)}
            placeholder="workspace id"
          />
        </label>
        <label>
          Relative path
          <input
            value={registerRelativePath}
            onChange={(event) => setRegisterRelativePath(event.target.value)}
            placeholder="path/inside/workspace"
          />
        </label>
        <label>
          Expected SHA-256 (content-match only)
          <input
            value={registerExpectedSha}
            onChange={(event) => setRegisterExpectedSha(event.target.value)}
            placeholder="64-hex digest"
          />
        </label>
        <div className="rail-actions rail-form-span">
          <button type="submit" className="primary-action" disabled={busy}>
            Register test
          </button>
        </div>
      </form>
    </section>
  );
}
