import type { MissionRunResult } from "../lib/bridge";
import { StatusMarker } from "./StatusMarker";

/**
 * Section 16 evidence-status vocabulary — every condition the typed graph is
 * able to render. This local slice only ever produces ACCEPTED (deterministic
 * checks bound to the final source) or, before a run, MISSING. The remaining
 * conditions are shown as a legend so the vocabulary is visible without
 * claiming any current node holds them.
 */
const EVIDENCE_VOCABULARY: ReadonlyArray<readonly [string, string]> = [
  ["Accepted", "Deterministic check, bound to the final source revision."],
  ["Weak", "Model assertion only; never independently checked."],
  ["Conflicting", "Two authorized verifiers disagree."],
  ["Stale", "The source changed after the evidence was produced."],
  ["Pending", "Verification has not yet completed."],
  ["Invalidated", "A later change revoked previously accepted evidence."],
  ["Missing", "A required claim has no evidence."],
];

/**
 * The verifier each desktop predicate actually ran
 * (mission_runner.rs::run_local_mission). Unknown claim ids fall back to the
 * honest generic label rather than inventing a verifier.
 */
const VERIFIER_BY_CLAIM: Record<string, string> = {
  build: "/usr/bin/git diff --check",
  tests: "byte-exact content match",
};

interface EvidenceGraphProps {
  result: MissionRunResult | null;
}

export function EvidenceGraph({ result }: EvidenceGraphProps) {
  if (!result) {
    return (
      <section className="evidence-graph is-empty" aria-labelledby="evidence-graph-heading">
        <span className="section-index">EVIDENCE GRAPH / MISSING</span>
        <h3 id="evidence-graph-heading">No evidence graph yet</h3>
        <p>
          The typed graph binds goal, claims, evidence digests, and the final source revision once
          the kernel commits completion. Model prose never creates a node.
        </p>
      </section>
    );
  }

  return (
    <section className="evidence-graph" aria-labelledby="evidence-graph-heading">
      <header className="panel-heading">
        <div>
          <span className="section-index">EVIDENCE GRAPH / TYPED</span>
          <h3 id="evidence-graph-heading">Evidence graph</h3>
        </div>
        <span className="proof-strength" aria-label="Proof strength deterministic check">
          DETERMINISTIC CHECK
        </span>
      </header>

      <div className="evidence-tree">
        <article className="evidence-root">
          <span className="evidence-branch-label">MISSION GOAL</span>
          <strong>Witnessed local change</strong>
          <code aria-label="Mission identifier">{result.missionId}</code>
        </article>

        <ul className="evidence-claims" aria-label="Required completion claims">
          {result.claims.map((claim) => (
            <li className="evidence-claim" key={claim.id}>
              <div className="evidence-claim-head">
                <strong>CLAIM · {claim.id.toUpperCase()}</strong>
                <StatusMarker status={claim.status} compact />
              </div>
              <p className="evidence-verifier">
                {VERIFIER_BY_CLAIM[claim.id] ?? "deterministic check"}
              </p>
              <dl className="evidence-leaves">
                <div>
                  <dt>Evidence digest</dt>
                  <dd>
                    <code aria-label={`${claim.id} evidence SHA-256 digest`}>
                      {claim.evidenceDigest}
                    </code>
                  </dd>
                </div>
                <div>
                  <dt>Bound source</dt>
                  <dd>
                    <code aria-label={`${claim.id} bound source SHA-256 digest`}>
                      {result.sourceDigest}
                    </code>
                  </dd>
                </div>
              </dl>
            </li>
          ))}
        </ul>

        <article className="evidence-root">
          <span className="evidence-branch-label">LEDGER HEAD</span>
          <code aria-label="Ledger head SHA-256 digest">{result.ledgerHead}</code>
        </article>
      </div>

      <ul className="evidence-legend" aria-label="Evidence status vocabulary">
        {EVIDENCE_VOCABULARY.map(([label, meaning]) => (
          <li key={label}>
            <strong>{label}</strong>
            <span>{meaning}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
