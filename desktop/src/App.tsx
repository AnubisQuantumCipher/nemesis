import { useEffect, useState } from "react";

import { AuthorityReview } from "./components/AuthorityReview";
import { FirstLaunch } from "./components/FirstLaunch";
import { MissionCockpit } from "./components/MissionCockpit";
import { Sidebar } from "./components/Sidebar";
import { AgentsPanel } from "./components/rails/AgentsPanel";
import { AutomationsPanel } from "./components/rails/AutomationsPanel";
import { ChangesPanel } from "./components/rails/ChangesPanel";
import { IntegrationsPanel } from "./components/rails/IntegrationsPanel";
import { KnowledgePanel } from "./components/rails/KnowledgePanel";
import { SecurityPanel } from "./components/rails/SecurityPanel";
import { SkillsPanel } from "./components/rails/SkillsPanel";
import { TestsPanel } from "./components/rails/TestsPanel";
import { WorkspacesPanel } from "./components/rails/WorkspacesPanel";
import { isRailName, type NavigationName, type RailName } from "./data/navigation";
import {
  adoptRailMutation,
  cancelMission,
  compileMission,
  draftMission,
  draftRailMutation,
  getMissionStatus,
  getReplay,
  getSystemStatus,
  normalizeFailure,
  runMission,
  updateSettings,
  type CommandFailure,
  type CompiledMission,
  type DesktopSettings,
  type MissionDraftRequest,
  type MissionRunResult,
  type MissionRuntimeSnapshot,
  type RailMutationRequest,
  type ReplayResult,
  type SystemStatus,
} from "./lib/bridge";
import "./styles.css";

const idleRuntime: MissionRuntimeSnapshot = {
  running: false,
  phase: "IDLE",
  detail: "No local mission is running.",
  lastResult: null,
  error: null,
};

function applySettings(settings: DesktopSettings) {
  document.documentElement.dataset.textScale = settings.textScale;
  document.documentElement.dataset.reduceMotion = String(settings.reduceMotion);
}

export default function App() {
  const [selected, setSelected] = useState<NavigationName>("Missions");
  const [system, setSystem] = useState<SystemStatus | null>(null);
  const [firstLaunchDismissed, setFirstLaunchDismissed] = useState(false);
  const [contractPath, setContractPath] = useState("");
  const [compiled, setCompiled] = useState<CompiledMission | null>(null);
  const [reviewing, setReviewing] = useState(false);
  const [compiling, setCompiling] = useState(false);
  const [drafting, setDrafting] = useState(false);
  const [savingSettings, setSavingSettings] = useState(false);
  const [runtime, setRuntime] = useState<MissionRuntimeSnapshot>(idleRuntime);
  const [result, setResult] = useState<MissionRunResult | null>(null);
  const [replay, setReplay] = useState<ReplayResult | null>(null);
  const [replayLoading, setReplayLoading] = useState(false);
  const [error, setError] = useState<CommandFailure | null>(null);
  const [railMission, setRailMission] = useState<RailName | null>(null);
  const [stateVersion, setStateVersion] = useState(0);
  const [composerSeed, setComposerSeed] = useState("");

  useEffect(() => {
    let current = true;
    getSystemStatus()
      .then((status) => {
        if (!current) {
          return;
        }
        setSystem(status);
        setResult(status.lastMission);
        applySettings(status.settings);
      })
      .catch((cause) => {
        if (current) {
          setError(normalizeFailure(cause));
        }
      });
    return () => {
      current = false;
    };
  }, []);

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setReviewing(false);
        return;
      }
      if (!event.metaKey || event.ctrlKey || event.altKey) {
        return;
      }
      const destination: NavigationName | undefined = {
        "1": "Home",
        "2": "Missions",
        "3": "Workspaces",
        "4": "Agents",
        "5": "Changes",
        "6": "Tests",
        "7": "Knowledge",
        "8": "Skills",
        "9": "Automations",
        "0": "Integrations",
        ",": "Settings",
      }[event.key] as NavigationName | undefined;
      if (destination) {
        event.preventDefault();
        setReviewing(false);
        setSelected(destination);
      }
    };
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, []);

  useEffect(() => {
    if (!runtime.running) {
      return;
    }
    const timer = window.setInterval(() => {
      void getMissionStatus()
        .then((snapshot) => {
          setRuntime(snapshot);
          if (snapshot.lastResult) {
            setResult(snapshot.lastResult);
          }
          if (snapshot.error) {
            setError(snapshot.error);
          }
        })
        .catch((cause) => setError(normalizeFailure(cause)));
    }, 250);
    return () => window.clearInterval(timer);
  }, [runtime.running]);

  useEffect(() => {
    if (selected !== "Replay") {
      return;
    }
    let current = true;
    setReplayLoading(true);
    setError(null);
    getReplay()
      .then((loaded) => {
        if (current) {
          setReplay(loaded);
        }
      })
      .catch((cause) => {
        if (current) {
          setReplay(null);
          setError(normalizeFailure(cause));
        }
      })
      .finally(() => {
        if (current) {
          setReplayLoading(false);
        }
      });
    return () => {
      current = false;
    };
  }, [selected]);

  async function draftContract(request: MissionDraftRequest) {
    setRailMission(null);
    setDrafting(true);
    setError(null);
    setCompiled(null);
    try {
      const drafted = await draftMission(request);
      setContractPath(drafted.path);
      setCompiled(drafted.compiled);
    } catch (cause) {
      setError(normalizeFailure(cause));
    } finally {
      setDrafting(false);
    }
  }

  async function compileContract() {
    setRailMission(null);
    setCompiling(true);
    setError(null);
    setCompiled(null);
    try {
      setCompiled(await compileMission(contractPath.trim()));
    } catch (cause) {
      setError(normalizeFailure(cause));
    } finally {
      setCompiling(false);
    }
  }

  async function beginRailMutation(request: RailMutationRequest) {
    setDrafting(true);
    setError(null);
    setCompiled(null);
    try {
      const drafted = await draftRailMutation(request);
      setContractPath(drafted.path);
      setCompiled(drafted.compiled);
      setRailMission(request.rail as RailName);
      setReviewing(true);
    } catch (cause) {
      setError(normalizeFailure(cause));
    } finally {
      setDrafting(false);
    }
  }

  async function authorizeAndRun() {
    if (!compiled) {
      return;
    }
    setReviewing(false);
    setError(null);
    setRuntime({
      ...idleRuntime,
      running: true,
      phase: "STARTING",
      detail: "Re-reading the exact reviewed local contract.",
    });
    try {
      const completed = await runMission(
        contractPath.trim(),
        compiled.contractDigest,
        compiled.actionDigest,
      );
      setResult(completed);
      setRuntime({
        running: false,
        phase: "COMPLETE",
        detail: "Receipt and authoritative replay are current.",
        lastResult: completed,
        error: null,
      });
      if (railMission) {
        const destination = railMission;
        try {
          await adoptRailMutation(completed.missionId);
          setStateVersion((version) => version + 1);
          setSelected(destination);
        } catch (cause) {
          setError(normalizeFailure(cause));
          setSelected("Evidence");
        } finally {
          setRailMission(null);
        }
      } else {
        setSelected("Evidence");
      }
    } catch (cause) {
      const failure = normalizeFailure(cause);
      setError(failure);
      setRuntime({
        running: false,
        phase: "REFUSED",
        detail: failure.message,
        lastResult: null,
        error: failure,
      });
    }
  }

  async function cancelCurrentMission() {
    try {
      setRuntime(await cancelMission());
    } catch (cause) {
      setError(normalizeFailure(cause));
    }
  }

  async function saveDesktopSettings(settings: DesktopSettings) {
    if (!system) {
      return;
    }
    setSavingSettings(true);
    setError(null);
    try {
      const persisted = await updateSettings(settings);
      setSystem({ ...system, settings: persisted });
      applySettings(persisted);
    } catch (cause) {
      setError(normalizeFailure(cause));
    } finally {
      setSavingSettings(false);
    }
  }

  if (!system) {
    return (
      <main className="startup-state">
        <span className="section-index">LOCAL HOME / INITIALIZING</span>
        <h1>NEMESIS Desktop</h1>
        {error ? (
          <div className="runtime-error" role="alert">
            <strong>{error.code}</strong>
            <span>{error.message}</span>
            <p>{error.recovery}</p>
          </div>
        ) : (
          <p>Validating bundled components and initializing private local state…</p>
        )}
      </main>
    );
  }

  const showFirstLaunch = system.firstLaunch && !firstLaunchDismissed;

  return (
    <main
      className="app-frame"
      data-text-scale={system.settings.textScale}
      data-reduce-motion={system.settings.reduceMotion}
    >
      <Sidebar selected={selected} onSelect={setSelected} />

      <section className="workspace-shell">
        <header className="command-bar">
          <div className="window-title">
            <span className="command-chevron" aria-hidden="true">//</span>
            <strong>{showFirstLaunch ? "FIRST LAUNCH" : selected.toUpperCase()}</strong>
            <span>LOCAL CONTROL PLANE</span>
          </div>
          <div className="system-health" aria-label="Local system health">
            <span className={system.ready ? "health-dot is-ready" : "health-dot"} aria-hidden="true" />
            <strong>CORE {system.core}</strong>
            <span>KERNEL {system.kernel}</span>
            <span>UPDATES DISABLED</span>
            <span>NET DENY</span>
          </div>
        </header>

        <div className="workspace-content" id="workspace-content" tabIndex={-1}>
          {showFirstLaunch ? (
            <FirstLaunch
              system={system}
              onBegin={(workspace) => {
                setFirstLaunchDismissed(true);
                setComposerSeed(workspace);
                setSelected("Missions");
              }}
            />
          ) : reviewing && compiled ? (
            <AuthorityReview
              compiled={compiled}
              running={runtime.running}
              onAuthorize={() => void authorizeAndRun()}
              onClose={() => setReviewing(false)}
            />
          ) : isRailName(selected) ? (
            selected === "Workspaces" ? (
              <WorkspacesPanel
                stateVersion={stateVersion}
                busy={runtime.running || drafting}
                onBeginMutation={(request) => void beginRailMutation(request)}
              />
            ) : selected === "Agents" ? (
              <AgentsPanel
                stateVersion={stateVersion}
                busy={runtime.running || drafting}
                onBeginMutation={(request) => void beginRailMutation(request)}
              />
            ) : selected === "Changes" ? (
              <ChangesPanel
                stateVersion={stateVersion}
                busy={runtime.running || drafting}
                onBeginMutation={(request) => void beginRailMutation(request)}
              />
            ) : selected === "Tests" ? (
              <TestsPanel
                stateVersion={stateVersion}
                busy={runtime.running || drafting}
                onBeginMutation={(request) => void beginRailMutation(request)}
              />
            ) : selected === "Knowledge" ? (
              <KnowledgePanel
                stateVersion={stateVersion}
                busy={runtime.running || drafting}
                onBeginMutation={(request) => void beginRailMutation(request)}
              />
            ) : selected === "Skills" ? (
              <SkillsPanel
                stateVersion={stateVersion}
                busy={runtime.running || drafting}
                onBeginMutation={(request) => void beginRailMutation(request)}
              />
            ) : selected === "Automations" ? (
              <AutomationsPanel
                stateVersion={stateVersion}
                busy={runtime.running || drafting}
                onBeginMutation={(request) => void beginRailMutation(request)}
              />
            ) : selected === "Integrations" ? (
              <IntegrationsPanel
                stateVersion={stateVersion}
                busy={runtime.running || drafting}
                onBeginMutation={(request) => void beginRailMutation(request)}
              />
            ) : (
              <SecurityPanel
                stateVersion={stateVersion}
                busy={runtime.running || drafting}
                onBeginMutation={(request) => void beginRailMutation(request)}
              />
            )
          ) : (
            <MissionCockpit
              selected={selected}
              system={system}
              contractPath={contractPath}
              compiled={compiled}
              runtime={runtime}
              result={result}
              replay={replay}
              replayLoading={replayLoading}
              compiling={compiling}
              drafting={drafting}
              savingSettings={savingSettings}
              error={error}
              onContractPath={setContractPath}
              onCompile={() => void compileContract()}
              onDraft={draftContract}
              onReview={() => setReviewing(true)}
              onCancel={() => void cancelCurrentMission()}
              onSaveSettings={saveDesktopSettings}
              seedWorkspace={composerSeed}
              onSeedConsumed={() => setComposerSeed("")}
            />
          )}
        </div>

        <footer className="telemetry-bar" aria-label="Runtime status">
          <span><b>WORKER</b> {runtime.running ? "ACTIVE" : "IDLE"}</span>
          <span><b>SANDBOX</b> {system.sandbox.replaceAll("_", " ")}</span>
          <span><b>NETWORK</b> {system.network.replaceAll("_", " ")}</span>
          <span><b>LEDGER</b> {result?.sequence ?? "NONE"}</span>
          <span><b>EVIDENCE</b> {result ? "CURRENT" : "NONE"}</span>
          <span className="telemetry-time">v{system.appVersion}</span>
        </footer>
      </section>
    </main>
  );
}
