import { useEffect, useState } from "react";

import {
  normalizeFailure,
  securitySnapshot,
  type CommandFailure,
  type SecuritySnapshot,
} from "../../lib/bridge";
import type { RailPanelProps } from "./props";

function digest12(value: unknown): string {
  return String(value ?? "").slice(0, 12);
}

export function SecurityPanel({ stateVersion }: RailPanelProps) {
  const [snapshot, setSnapshot] = useState<SecuritySnapshot | null>(null);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [refreshCount, setRefreshCount] = useState(0);

  useEffect(() => {
    let current = true;
    setError(null);
    securitySnapshot()
      .then((response) => {
        if (current) {
          setSnapshot(response);
        }
      })
      .catch((cause) => {
        if (current) {
          setSnapshot(null);
          setError(normalizeFailure(cause));
        }
      });
    return () => {
      current = false;
    };
  }, [stateVersion, refreshCount]);

  const policyEntries = snapshot ? Object.entries(snapshot.policy) : [];
  const secretEntries = snapshot ? Object.entries(snapshot.secretsPosture) : [];

  return (
    <section className="rail-panel" aria-labelledby="security-heading">
      <span className="section-index">RAIL SC / AUTHORITY TRUTH SURFACE</span>
      <h2 id="security-heading">Security</h2>
      <p className="rail-doctrine">
        This rail reads the authority artifacts the kernel persisted — parent grants,
        one-shot approvals, the adoption chain — and the standing deny postures; corrupt
        records are shown, never filtered.
      </p>

      <div className="rail-actions">
        <button
          type="button"
          className="text-button"
          onClick={() => setRefreshCount((count) => count + 1)}
        >
          Refresh
        </button>
      </div>

      {error ? (
        <div className="rail-error" role="alert">
          <strong>{error.code}</strong>
          <span>{error.message}</span>
          <span>{error.recovery}</span>
        </div>
      ) : null}

      {snapshot === null && !error ? (
        <p className="rail-empty">Loading authority artifacts…</p>
      ) : null}

      {snapshot !== null ? (
        <>
          <span className="section-index">POLICY</span>
          {policyEntries.length === 0 ? (
            <p className="rail-empty">No policy posture persisted yet.</p>
          ) : (
            <div className="rail-table" role="list" aria-label="Standing policy">
              {policyEntries.map(([key, value]) => (
                <div className="rail-row" role="listitem" key={key}>
                  <span className="rail-row-id">{key}</span>
                  <span className="rail-row-meta">
                    <span
                      className={
                        value === "DENIED_BY_CONTRACT" ? "rail-tag is-verified" : undefined
                      }
                    >
                      {value}
                    </span>
                  </span>
                </div>
              ))}
            </div>
          )}

          <span className="section-index">SECRETS</span>
          {secretEntries.length === 0 ? (
            <p className="rail-empty">No secrets posture persisted yet.</p>
          ) : (
            <div className="rail-table" role="list" aria-label="Secrets posture">
              {secretEntries.map(([key, value]) => (
                <div className="rail-row" role="listitem" key={key}>
                  <span className="rail-row-id">{key}</span>
                  <span className="rail-row-meta">
                    <span className={value === "NONE" ? "rail-tag is-verified" : undefined}>
                      {value}
                    </span>
                  </span>
                </div>
              ))}
            </div>
          )}

          <span className="section-index">APPROVALS</span>
          <p className="rail-row-meta">
            ARMED {snapshot.approvalTally.armed} / CONSUMED {snapshot.approvalTally.consumed} /
            CORRUPT {snapshot.approvalTally.corrupt}
          </p>
          {snapshot.approvals.length === 0 ? (
            <p className="rail-empty">No one-shot approvals persisted yet.</p>
          ) : (
            <div className="rail-table" role="list" aria-label="One-shot approvals">
              {snapshot.approvals.map((approval, index) => {
                const status = String(approval.status ?? "CORRUPT");
                const errorText = approval.error ? String(approval.error) : null;
                return (
                  <div
                    className="rail-row"
                    role="listitem"
                    key={String(approval.approvalId ?? index)}
                  >
                    <span className="rail-row-id">{String(approval.approvalId ?? "")}</span>
                    <span className="rail-row-meta">
                      <span>{String(approval.missionId ?? "")}</span>
                      <span>action {digest12(approval.actionDigest)}…</span>
                      <span
                        className={
                          status === "ARMED"
                            ? "rail-tag is-armed"
                            : status === "CONSUMED"
                              ? "rail-tag is-verified"
                              : "rail-tag is-corrupt"
                        }
                      >
                        {status}
                      </span>
                      {errorText ? <span>{errorText}</span> : null}
                    </span>
                  </div>
                );
              })}
            </div>
          )}

          <span className="section-index">GRANTS</span>
          {snapshot.grants.length === 0 ? (
            <p className="rail-empty">No grants persisted yet.</p>
          ) : (
            <div className="rail-table" role="list" aria-label="Parent grants">
              {snapshot.grants.map((grant, index) => {
                const status = String(grant.status ?? "CORRUPT");
                const errorText = grant.error ? String(grant.error) : null;
                const operations = Array.isArray(grant.operations)
                  ? grant.operations.map(String).join(", ")
                  : String(grant.operations ?? "");
                return (
                  <div
                    className="rail-row"
                    role="listitem"
                    key={String(grant.grantId ?? index)}
                  >
                    <span className="rail-row-id">{String(grant.grantId ?? "")}</span>
                    <span className="rail-row-meta">
                      <span>{String(grant.missionId ?? "")}</span>
                      <span>{operations}</span>
                      <span>scope {digest12(grant.scopeDigest)}…</span>
                      <span>max {String(grant.maximumBytes ?? "")} bytes</span>
                      <span
                        className={
                          status === "ACTIVE" ? "rail-tag is-armed" : "rail-tag is-corrupt"
                        }
                      >
                        {status}
                      </span>
                      {errorText ? <span>{errorText}</span> : null}
                    </span>
                  </div>
                );
              })}
            </div>
          )}

          <span className="section-index">ADOPTION CHAIN</span>
          <p className="rail-row-meta">
            <span>
              length {snapshot.adoptionChain.length} / head {digest12(snapshot.adoptionChain.head)}…
            </span>
            <span
              className={
                snapshot.adoptionChain.verified ? "rail-tag is-verified" : "rail-tag is-corrupt"
              }
            >
              {snapshot.adoptionChain.verified ? "VERIFIED" : "UNVERIFIED"}
            </span>
          </p>
          {snapshot.adoptionChain.records.length === 0 ? (
            <p className="rail-empty">No adoption records persisted yet.</p>
          ) : (
            <div className="rail-table" role="list" aria-label="Adoption chain">
              {snapshot.adoptionChain.records.map((record) => (
                <div className="rail-row" role="listitem" key={record.entryHash}>
                  <span className="rail-row-id">
                    {record.sequence} {record.rail} {record.relativePath}
                  </span>
                  <span className="rail-row-meta">
                    <span>{record.missionId}</span>
                    <span>content {digest12(record.contentDigest)}…</span>
                    <span>state {digest12(record.stateCommit)}…</span>
                  </span>
                </div>
              ))}
            </div>
          )}
        </>
      ) : null}
    </section>
  );
}
