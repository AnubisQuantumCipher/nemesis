import type { MissionRunResult } from "../lib/bridge";
import { StatusMarker } from "./StatusMarker";

interface EvidencePanelProps {
  result: MissionRunResult | null;
}

export function EvidencePanel({ result }: EvidencePanelProps) {
  return (
    <section className="evidence-court" aria-labelledby="court-heading">
      <header className="panel-heading">
        <div>
          <span className="section-index">COURT / FINAL</span>
          <h2 id="court-heading">Completion court</h2>
        </div>
        <span className={result ? "court-state is-verified" : "court-state"}>
          {result ? "MISSION VERIFIED" : "AWAITING EVIDENCE"}
        </span>
      </header>

      <div className="court-grid">
        <article className="court-column">
          <span>Required claims</span>
          {(result?.claims ?? [
            { id: "build", status: "UNKNOWN" as const, evidenceDigest: "No accepted evidence" },
            { id: "tests", status: "UNKNOWN" as const, evidenceDigest: "No accepted evidence" },
          ]).map((claim) => (
            <div className="claim-row" key={claim.id}>
              <div>
                <strong>{claim.id.toUpperCase()}</strong>
                <code>{claim.evidenceDigest}</code>
              </div>
              <StatusMarker status={claim.status} compact />
            </div>
          ))}
        </article>

        <article className="court-column source-binding">
          <span>Final source binding</span>
          <strong>{result ? "BOUND" : "UNBOUND"}</strong>
          <code>{result?.sourceDigest ?? "Source digest not yet established"}</code>
          <dl>
            <div>
              <dt>Mission sequence</dt>
              <dd>{result?.sequence ?? "—"}</dd>
            </div>
            <div>
              <dt>Ledger head</dt>
              <dd>{result?.ledgerHead.slice(0, 16) ?? "—"}</dd>
            </div>
          </dl>
        </article>

        <article className="court-column kernel-decision">
          <span>Kernel decision</span>
          <div className={result ? "kernel-seal is-active" : "kernel-seal"} aria-hidden="true">
            N
          </div>
          <strong>{result ? "COMPLETE" : "NOT ENTERED"}</strong>
          <p>
            {result
              ? "Every mandatory claim is current, accepted, and bound to final source."
              : "Worker prose cannot advance this state."}
          </p>
        </article>
      </div>

      {result ? (
        <div className="tamper-result">
          <span className="tamper-mark" aria-hidden="true">×</span>
          <div>
            <strong>TAMPER REJECTED</strong>
            <p>Independent verifier refused the one-byte mutation.</p>
          </div>
          <code>{result.artifactDirectory}</code>
        </div>
      ) : null}
    </section>
  );
}
