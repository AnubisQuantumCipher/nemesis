import type { ReplayResult } from "../lib/bridge";

interface ReplayPanelProps {
  replay: ReplayResult | null;
  loading: boolean;
}

export function ReplayPanel({ replay, loading }: ReplayPanelProps) {
  return (
    <section className="replay-panel" aria-labelledby="replay-heading">
      <header className="panel-heading">
        <div>
          <span className="section-index">REPLAY / RECORDED CAUSALITY</span>
          <h2 id="replay-heading">Mission replay</h2>
        </div>
        <span className={replay ? "court-state is-verified" : "court-state"}>
          {loading ? "LOADING LEDGER" : replay ? "CHAIN VERIFIED" : "NO REPLAY LOADED"}
        </span>
      </header>

      <div className="replay-assurance">
        <div>
          <span className={replay?.exactStateReconstruction ? "replay-check is-active" : "replay-check"} aria-hidden="true">◇</span>
          <strong>Exact state reconstruction</strong>
          <p>Committed sequence, state, payload, and hash chain.</p>
        </div>
        <div>
          <span className="replay-check" aria-hidden="true">≈</span>
          <strong>Model re-execution is comparative</strong>
          <p>Nondeterministic tokens are never labeled exact replay.</p>
        </div>
      </div>

      <div className="replay-timeline" aria-label="Authoritative event timeline">
        {(replay?.events ?? []).map((event) => (
          <article className="replay-event" key={event.sequence}>
            <span className="replay-sequence">EVENT {String(event.sequence).padStart(4, "0")}</span>
            <div className="replay-connector" aria-hidden="true" />
            <div>
              <strong>Kind {event.kindCode} · State {event.stateCode}</strong>
              <code>{event.eventHash}</code>
            </div>
          </article>
        ))}
        {!loading && !replay ? (
          <p className="replay-empty">Run a witnessed mission to record a replayable ledger.</p>
        ) : null}
      </div>

      {replay ? (
        <footer className="replay-head">
          <span>CHAIN HEAD</span>
          <code>{replay.head}</code>
        </footer>
      ) : null}
    </section>
  );
}
