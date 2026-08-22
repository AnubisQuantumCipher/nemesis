import type { MissionRunResult } from "../lib/bridge";
import { StatusMarker } from "./StatusMarker";

interface EvidencePanelProps {
  result: MissionRunResult | null;
}

export function EvidencePanel({ result }: EvidencePanelProps) {
  if (!result) {
    return (
      <section className="evidence-court empty-state" aria-labelledby="court-heading">
        <span className="section-index">COURT / EMPTY</span>
        <h2 id="court-heading">No accepted mission evidence</h2>
        <p>Compile, review, and run a local contract. Worker prose never creates evidence.</p>
      </section>
    );
  }

  return (
    <section className="evidence-court" aria-labelledby="court-heading">
      <header className="panel-heading">
        <div>
          <span className="section-index">COURT / CURRENT LOCAL MISSION</span>
          <h2 id="court-heading">Completion court</h2>
        </div>
        <span className="court-state is-verified">MISSION VERIFIED</span>
      </header>

      <div className="court-grid">
        <article className="court-column">
          <span>Required claims</span>
          {result.claims.map((claim) => (
            <div className="claim-row" key={claim.id}>
              <div>
                <strong>{claim.id.toUpperCase()}</strong>
                <code aria-label={`${claim.id} evidence SHA-256 digest`}>{claim.evidenceDigest}</code>
              </div>
              <StatusMarker status={claim.status} compact />
            </div>
          ))}
        </article>

        <article className="court-column source-binding">
          <span>Final source binding</span>
          <strong>BOUND</strong>
          <code aria-label="Final source SHA-256 digest">{result.sourceDigest}</code>
          <dl>
            <div>
              <dt>Mission sequence</dt>
              <dd>{result.sequence}</dd>
            </div>
            <div>
              <dt>Ledger head</dt>
              <dd><code aria-label="Ledger head SHA-256 digest">{result.ledgerHead}</code></dd>
            </div>
          </dl>
        </article>

        <article className="court-column kernel-decision">
          <span>Kernel decision</span>
          <div className="kernel-seal is-active" aria-hidden="true">N</div>
          <strong>COMPLETE</strong>
          <p>Every mandatory claim is current, accepted, and bound to final source.</p>
        </article>
      </div>

      <div className="tamper-result">
        <span className="tamper-mark" aria-hidden="true">×</span>
        <div>
          <strong>TAMPER REJECTED</strong>
          <p>Standalone verifier refused the one-byte receipt mutation.</p>
        </div>
        <div className="local-paths">
          <span>Local evidence</span>
          <code aria-label="Local evidence directory">{result.artifactDirectory}</code>
          <span>Isolated lane</span>
          <code aria-label="Local isolated lane path">{result.lanePath}</code>
        </div>
      </div>
    </section>
  );
}
