import { useEffect, useState } from "react";

import {
  changesSnapshot,
  normalizeFailure,
  type ChangesSnapshot,
  type CommandFailure,
  type LaneChange,
} from "../../lib/bridge";
import type { RailPanelProps } from "./props";

export function ChangesPanel({ stateVersion }: RailPanelProps) {
  const [snapshot, setSnapshot] = useState<ChangesSnapshot | null>(null);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [refreshToken, setRefreshToken] = useState(0);

  useEffect(() => {
    let current = true;
    setError(null);
    changesSnapshot()
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
  }, [stateVersion, refreshToken]);

  return (
    <section className="rail-panel" aria-labelledby="changes-heading">
      <span className="section-index">RAIL CH / LANE DIFFS VS CANONICAL</span>
      <h2 id="changes-heading">Changes</h2>
      <p className="rail-doctrine">
        Every mission writes only to an isolated lane; this view computes lane state live
        against canonical source; corrupt lanes are surfaced, never hidden.
      </p>

      <div className="rail-actions">
        <button
          type="button"
          className="text-button"
          onClick={() => setRefreshToken((token) => token + 1)}
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
        <p className="rail-empty">Loading lane state…</p>
      ) : null}

      {snapshot !== null && snapshot.lanes.length === 0 ? (
        <p className="rail-empty">No lanes. Missions create isolated lanes on authorization.</p>
      ) : null}

      {snapshot !== null && snapshot.lanes.length > 0 ? (
        <>
          <p className="rail-doctrine">
            {snapshot.laneCount} lane{snapshot.laneCount === 1 ? "" : "s"} tracked
          </p>
          <div className="rail-table" role="list" aria-label="Mission lanes">
            {snapshot.lanes.map((lane) => (
              <LaneRow lane={lane} key={lane.missionId} />
            ))}
          </div>
        </>
      ) : null}
    </section>
  );
}

function LaneRow({ lane }: { lane: LaneChange }) {
  const corrupt = lane.status === "CORRUPT";
  return (
    <div className="rail-row" role="listitem">
      <span className="rail-row-id">{lane.missionId}</span>
      <span className="rail-row-meta">
        {lane.branch ? <span>{lane.branch}</span> : null}
        {lane.head ? <span>{lane.head.slice(0, 12)}</span> : null}
        {lane.workspace ? <span>{lane.workspace}</span> : null}
        {lane.relativePath ? <span>{lane.relativePath}</span> : null}
        {lane.diffShortstat ? <span>{lane.diffShortstat}</span> : null}
        {typeof lane.dirtyCount === "number" && lane.dirtyCount > 0 ? (
          <span className="rail-tag is-armed">DIRTY {lane.dirtyCount}</span>
        ) : null}
        {lane.dirtyCount === 0 ? <span className="rail-tag is-verified">CLEAN</span> : null}
        {corrupt ? <span className="rail-tag is-corrupt">CORRUPT</span> : null}
        {corrupt && lane.error ? <span>{lane.error}</span> : null}
        {lane.dirtyFiles && lane.dirtyFiles.length > 0
          ? lane.dirtyFiles.map((file) => (
              <span key={`${file.code} ${file.path}`}>
                {file.code} {file.path}
              </span>
            ))
          : null}
      </span>
    </div>
  );
}
