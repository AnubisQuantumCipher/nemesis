import { useEffect, useState } from "react";

import { AuthorityReview } from "./components/AuthorityReview";
import { MissionCockpit } from "./components/MissionCockpit";
import { Sidebar } from "./components/Sidebar";
import type { NavigationName } from "./data/navigation";
import {
  getSystemStatus,
  runWitnessedMission,
  type MissionRunResult,
  type SystemStatus,
} from "./lib/bridge";
import "./styles.css";

export default function App() {
  const [selected, setSelected] = useState<NavigationName>("Missions");
  const [system, setSystem] = useState<SystemStatus | null>(null);
  const [reviewing, setReviewing] = useState(false);
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<MissionRunResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let current = true;
    getSystemStatus()
      .then((status) => {
        if (current) {
          setSystem(status);
        }
      })
      .catch(() => {
        if (current) {
          setError("Native Core bridge unavailable. Launch NEMESIS through the local Tauri app.");
        }
      });
    return () => {
      current = false;
    };
  }, []);

  async function authorizeAndRun() {
    setRunning(true);
    setError(null);
    setReviewing(false);
    try {
      const mission = await runWitnessedMission();
      setResult(mission);
      setSelected("Evidence");
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      setError(`Mission refused or blocked: ${message}`);
    } finally {
      setRunning(false);
    }
  }

  return (
    <main className="app-frame">
      <Sidebar selected={selected} onSelect={setSelected} />

      <section className="workspace-shell">
        <header className="command-bar">
          <div className="window-title">
            <span className="command-chevron" aria-hidden="true">//</span>
            <strong>{selected.toUpperCase()}</strong>
            <span>LOCAL CONTROL PLANE</span>
          </div>
          <div className="system-health" aria-label="Local system health">
            <span className={system?.ready ? "health-dot is-ready" : "health-dot"} aria-hidden="true" />
            <strong>{system?.ready ? "CORE READY" : "CORE CHECK"}</strong>
            <span>KERNEL {system?.kernel ?? "PENDING"}</span>
            <span>NET DENY</span>
          </div>
        </header>

        <div className="workspace-content">
          {reviewing ? (
            <AuthorityReview
              running={running}
              onAuthorize={authorizeAndRun}
              onClose={() => setReviewing(false)}
            />
          ) : (
            <MissionCockpit
              selected={selected}
              system={system}
              result={result}
              running={running}
              error={error}
              onReview={() => setReviewing(true)}
            />
          )}
        </div>

        <footer className="telemetry-bar" aria-label="Runtime telemetry">
          <span><b>WORKER</b> {running ? "ACTIVE" : "IDLE"}</span>
          <span><b>SANDBOX</b> WORKSPACE SAFE</span>
          <span><b>NETWORK</b> DENIED</span>
          <span><b>LEDGER</b> {result?.sequence ?? "—"}</span>
          <span><b>EVIDENCE</b> {result ? "CURRENT" : "PENDING"}</span>
          <span className="telemetry-time">AUTONOMY UNDER CONTROL.</span>
        </footer>
      </section>
    </main>
  );
}
