import { useEffect, useRef } from "react";

import type { CompiledMission } from "../lib/bridge";

interface AuthorityReviewProps {
  compiled: CompiledMission;
  running: boolean;
  onAuthorize: () => void;
  onClose: () => void;
}

export function AuthorityReview({
  compiled,
  running,
  onAuthorize,
  onClose,
}: AuthorityReviewProps) {
  const headingRef = useRef<HTMLHeadingElement>(null);

  useEffect(() => {
    headingRef.current?.focus();
  }, []);

  return (
    <section className="authority-review" aria-labelledby="authority-heading">
      <header className="panel-heading authority-heading">
        <div>
          <span className="section-index">AUTH / EXACT LOCAL CONTRACT</span>
          <h2 id="authority-heading" ref={headingRef} tabIndex={-1}>
            Authority review
          </h2>
        </div>
        <button type="button" className="text-button" onClick={onClose} disabled={running}>
          Close
        </button>
      </header>

      <div className="digest-grid">
        <div>
          <span>CONTRACT SHA-256</span>
          <code aria-label="Contract SHA-256 digest">{compiled.contractDigest}</code>
        </div>
        <div>
          <span>ACTION SHA-256</span>
          <code aria-label="Action SHA-256 digest">{compiled.actionDigest}</code>
        </div>
      </div>

      <div className="authority-table" role="table" aria-label="Mission authority">
        <div className="authority-row" role="row">
          <strong role="cell">FILESYSTEM</strong>
          <span role="cell">{compiled.relativePath}</span>
          <b role="cell" className="decision-exact">WRITE EXACT</b>
        </div>
        <div className="authority-row" role="row">
          <strong role="cell">NETWORK</strong>
          <span role="cell">No worker egress</span>
          <b role="cell" className="decision-deny">NETWORK DENY</b>
        </div>
        <div className="authority-row" role="row">
          <strong role="cell">PUSH / PUBLISH</strong>
          <span role="cell">No remote mutation</span>
          <b role="cell" className="decision-deny">PUSH DENY</b>
        </div>
        <div className="authority-row" role="row">
          <strong role="cell">SECRETS</strong>
          <span role="cell">No worker secret view</span>
          <b role="cell" className="decision-deny">SECRETS DENY</b>
        </div>
      </div>

      <div className="invariant-grid">
        <div>
          <span>WORKSPACE</span>
          <strong>{compiled.workspace}</strong>
          <p>Base {compiled.baseRevision}</p>
        </div>
        <div>
          <span>WRITE BUDGET</span>
          <strong>{compiled.replacementBytes} / {compiled.maxWriteBytes} bytes</strong>
          <p>One existing regular UTF-8 file.</p>
        </div>
        <div>
          <span>RUNTIME BUDGET</span>
          <strong>{compiled.maxRuntimeSeconds} seconds</strong>
          <p>Output capped at {compiled.maxOutputBytes} bytes.</p>
        </div>
      </div>

      <details className="normalized-contract">
        <summary>Inspect normalized contract bytes</summary>
        <pre>{compiled.normalizedContract}</pre>
      </details>

      <footer className="authority-actions">
        <p>
          Authorization binds both digests above. The backend re-reads the file and refuses any
          changed byte before creating a lane.
        </p>
        <button type="button" className="primary-action" onClick={onAuthorize} disabled={running}>
          {running ? "MISSION RUNNING" : "Authorize and run"}
        </button>
      </footer>
    </section>
  );
}
