import type { EvidenceStatus } from "../lib/bridge";

interface StatusMarkerProps {
  status: EvidenceStatus;
  compact?: boolean;
}

export function StatusMarker({ status, compact = false }: StatusMarkerProps) {
  return (
    <span
      className={`status-marker status-${status.toLowerCase()}${compact ? " status-compact" : ""}`}
      aria-label={`Evidence status ${status}`}
    >
      <span className="status-glyph" aria-hidden="true" />
      {status}
    </span>
  );
}
