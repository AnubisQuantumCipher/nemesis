interface AuthorityReviewProps {
  running: boolean;
  onAuthorize: () => void;
  onClose: () => void;
}

const authorityRows = [
  ["Filesystem", "One disposable Git worktree", "WRITE / BOUNDED"],
  ["Network", "No worker egress", "DENY"],
  ["Secrets", "No worker secret view", "DENY"],
  ["Process", "Exact sandboxed worker and verifiers", "EXECUTE / EXACT"],
] as const;

export function AuthorityReview({ running, onAuthorize, onClose }: AuthorityReviewProps) {
  return (
    <section className="authority-review" aria-labelledby="authority-heading">
      <header className="panel-heading authority-heading">
        <div>
          <span className="section-index">AUTH / 01</span>
          <h2 id="authority-heading">Authority review</h2>
        </div>
        <button type="button" className="text-button" onClick={onClose}>
          Close
        </button>
      </header>

      <div className="contract-hash">
        <span>CONTRACT</span>
        <code>nemesis.mission/v1 · digest assigned at execution</code>
      </div>

      <div className="authority-table" role="table" aria-label="Mission authority">
        {authorityRows.map(([resource, scope, decision]) => (
          <div className="authority-row" role="row" key={resource}>
            <strong role="cell">{resource}</strong>
            <span role="cell">{scope}</span>
            <b role="cell" className={decision === "DENY" ? "decision-deny" : "decision-exact"}>
              {decision}
            </b>
          </div>
        ))}
      </div>

      <div className="invariant-grid">
        <div>
          <span className="invariant-icon" aria-hidden="true">×</span>
          <strong>Push disabled</strong>
          <p>No remote mutation authority.</p>
        </div>
        <div>
          <span className="invariant-icon" aria-hidden="true">×</span>
          <strong>Publish disabled</strong>
          <p>No release or registry authority.</p>
        </div>
        <div>
          <span className="invariant-icon" aria-hidden="true">◇</span>
          <strong>Worker cannot mark complete</strong>
          <p>Completion belongs to Kernel predicates.</p>
        </div>
      </div>

      <footer className="authority-actions">
        <p>Authorization binds this exact local mission contract. Any mutation requires review again.</p>
        <button
          type="button"
          className="primary-action"
          onClick={onAuthorize}
          disabled={running}
        >
          {running ? "MISSION RUNNING" : "Authorize and run"}
        </button>
      </footer>
    </section>
  );
}
