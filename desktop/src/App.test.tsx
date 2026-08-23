import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";
import {
  adoptRailMutation,
  cancelMission,
  changesSnapshot,
  compileMission,
  dispatchAutomation,
  draftMission,
  draftRailMutation,
  executeIntegrationPlugin,
  getMissionStatus,
  getReplay,
  getSystemStatus,
  railEntities,
  runMission,
  runRailTest,
  securitySnapshot,
  updateSettings,
} from "./lib/bridge";

vi.mock("./lib/bridge", () => ({
  adoptRailMutation: vi.fn(),
  cancelMission: vi.fn(),
  changesSnapshot: vi.fn(),
  compileMission: vi.fn(),
  dispatchAutomation: vi.fn(),
  draftMission: vi.fn(),
  draftRailMutation: vi.fn(),
  executeIntegrationPlugin: vi.fn(),
  getMissionStatus: vi.fn(),
  getReplay: vi.fn(),
  getSystemStatus: vi.fn(),
  normalizeFailure: (cause: unknown) => cause,
  railEntities: vi.fn(),
  runMission: vi.fn(),
  runRailTest: vi.fn(),
  securitySnapshot: vi.fn(),
  updateSettings: vi.fn(),
}))

const mockedStatus = vi.mocked(getSystemStatus);
const mockedCompile = vi.mocked(compileMission);
const mockedDraft = vi.mocked(draftMission);
const mockedRun = vi.mocked(runMission);
const mockedMissionStatus = vi.mocked(getMissionStatus);
const mockedCancel = vi.mocked(cancelMission);
const mockedReplay = vi.mocked(getReplay);
const mockedUpdateSettings = vi.mocked(updateSettings);
const mockedRailEntities = vi.mocked(railEntities);
const mockedChanges = vi.mocked(changesSnapshot);
const mockedSecurity = vi.mocked(securitySnapshot);
const mockedDraftRail = vi.mocked(draftRailMutation);
const mockedAdopt = vi.mocked(adoptRailMutation);
const mockedRunRailTest = vi.mocked(runRailTest);
const mockedDispatch = vi.mocked(dispatchAutomation);
const mockedExecutePlugin = vi.mocked(executeIntegrationPlugin);

const settings = {
  textScale: "standard" as const,
  reduceMotion: true,
};

const system = {
  ready: true,
  core: "READY",
  kernel: "AVAILABLE",
  runtime: "READY",
  contractSha256:
    "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779",
  localHome: "/tmp/nemesis-home",
  firstLaunch: false,
  schemaVersion: 1,
  appVersion: "0.2.0",
  sandbox: "WORKSPACE_SAFE_AVAILABLE",
  network: "DENIED_BY_CONTRACT",
  updates: "DISABLED_NO_AUTHENTICATED_UPDATER",
  settings,
  lastMission: null,
};

const compiled = {
  missionId: "mis_0123456789abcdefghij12",
  goal: "Replace one bounded UTF-8 file in an isolated lane.",
  workspace: "/tmp/repository",
  baseRevision: "aa".repeat(20),
  relativePath: "value.txt",
  expectedSha256: "bb".repeat(32),
  contentDigest: "cc".repeat(32),
  replacementBytes: 6,
  contractDigest: "dd".repeat(32),
  actionDigest: "ee".repeat(32),
  normalizedContract: "{\"schema\":\"nemesis.desktop-mission/v1\"}",
  capabilities: ["filesystem.modify:exact"],
  maxWriteBytes: 4096,
  maxRuntimeSeconds: 300,
  maxOutputBytes: 1048576,
};

const result = {
  status: "VERIFIED" as const,
  missionId: compiled.missionId,
  sequence: 12,
  sourceDigest: "44".repeat(32),
  ledgerHead: "55".repeat(32),
  artifactDirectory: "/tmp/nemesis-evidence",
  lanePath: "/tmp/nemesis-lane",
  claims: [
    { id: "build", status: "VERIFIED" as const, evidenceDigest: "66".repeat(32) },
    { id: "tests", status: "VERIFIED" as const, evidenceDigest: "77".repeat(32) },
  ],
  tamperVerdict: "REJECTED" as const,
  replayPath: "/tmp/nemesis-evidence/replay.json",
};

const idleRuntime = {
  running: false,
  phase: "IDLE",
  detail: "No local mission is running.",
  lastResult: null,
  error: null,
};

beforeEach(() => {
  mockedStatus.mockResolvedValue(system);
  mockedCompile.mockResolvedValue(compiled);
  mockedRun.mockResolvedValue(result);
  mockedMissionStatus.mockResolvedValue(idleRuntime);
  mockedCancel.mockResolvedValue({ ...idleRuntime, phase: "CANCELLING" });
  mockedDraft.mockResolvedValue({
    path: "/tmp/nemesis-home/drafts/mission.json",
    compiled,
  });
  mockedReplay.mockResolvedValue({
    verdict: "VERIFIED",
    finalStateCode: 8,
    head: "aa".repeat(32),
    exactStateReconstruction: true,
    exactModelReexecution: false,
    events: [
      { sequence: 1, kindCode: 0, stateCode: 1, eventHash: "bb".repeat(32) },
      { sequence: 2, kindCode: 3, stateCode: 4, eventHash: "cc".repeat(32) },
    ],
  });
  mockedUpdateSettings.mockImplementation(async (next) => next);
  mockedRailEntities.mockResolvedValue({ rail: "skills", entities: [] });
  mockedChanges.mockResolvedValue({ laneCount: 0, lanes: [] });
  mockedSecurity.mockResolvedValue({
    policy: { network: "DENIED_BY_CONTRACT" },
    secretsPosture: { stored: "NONE" },
    grants: [],
    approvals: [],
    approvalTally: { armed: 0, consumed: 0, corrupt: 0 },
    adoptionChain: { length: 0, head: "0".repeat(64), verified: true, records: [] },
  });
  mockedDraftRail.mockResolvedValue({
    path: "/tmp/nemesis-home/drafts/mission.json",
    compiled,
  });
  mockedAdopt.mockResolvedValue({
    schema: "nemesis.rail-adoption/v1",
    sequence: 1,
    missionId: "mis_" + "a".repeat(22),
    rail: "skills",
    relativePath: "skills/x.json",
    contentDigest: "d".repeat(64),
    contractDigest: "e".repeat(64),
    actionDigest: "f".repeat(64),
    stateCommit: "1".repeat(40),
    previous: "0".repeat(64),
    entryHash: "2".repeat(64),
  });
  mockedRunRailTest.mockResolvedValue({
    schema: "nemesis.test-run/v1",
    sequence: 1,
    subject: "t",
    verdict: "PASS",
    detail: {},
    recordedUnixSeconds: 0,
    previous: "0".repeat(64),
    entryHash: "3".repeat(64),
  });
  mockedDispatch.mockResolvedValue({
    schema: "nemesis.automation-run/v1",
    sequence: 1,
    subject: "a",
    verdict: "AUTHORIZED_DRAFT_ONLY",
    detail: {},
    recordedUnixSeconds: 0,
    previous: "0".repeat(64),
    entryHash: "4".repeat(64),
  });
  mockedExecutePlugin.mockResolvedValue({
    schema: "nemesis.plugin-run/v1",
    sequence: 1,
    subject: "p",
    verdict: "EXECUTED",
    detail: {},
    recordedUnixSeconds: 0,
    previous: "0".repeat(64),
    entryHash: "5".repeat(64),
  });
});

describe("NEMESIS Desktop production surface", () => {
  it("exposes all fourteen product destinations and truthful component state", async () => {
    render(<App />);
    await screen.findByRole("button", { name: "Home" });

    for (const name of [
      "Home",
      "Missions",
      "Workspaces",
      "Agents",
      "Changes",
      "Tests",
      "Knowledge",
      "Skills",
      "Automations",
      "Integrations",
      "Security",
      "Evidence",
      "Replay",
      "Settings",
    ]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
    await waitFor(() => expect(screen.getByText("CORE READY")).toBeInTheDocument());
    expect(screen.getByText("UPDATES DISABLED")).toBeInTheDocument();
    expect(screen.queryByText("PROVED_LOCAL")).not.toBeInTheDocument();
  });

  it("renders the governed Skills rail through the shared authority shell", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Skills" }));
    expect(await screen.findByRole("heading", { name: "Skills" })).toBeInTheDocument();
    await waitFor(() => expect(mockedRailEntities).toHaveBeenCalledWith("skills"));
  });

  it("supports arrow navigation and the native Settings accelerator", async () => {
    render(<App />);
    const home = await screen.findByRole("button", { name: "Home" });
    const missions = screen.getByRole("button", { name: "Missions" });
    home.focus();
    fireEvent.keyDown(home, { key: "ArrowDown" });
    expect(missions).toHaveFocus();

    fireEvent.keyDown(window, { key: ",", metaKey: true });
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });

  it("drafts an exact local contract without a terminal prerequisite", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Missions" }));
    await user.click(screen.getByRole("button", { name: "Review contract" }));
    await user.type(screen.getByRole("textbox", { name: "Mission goal" }), "Replace the value");
    await user.type(screen.getByRole("textbox", { name: "Workspace path" }), "/tmp/repository");
    await user.type(screen.getByRole("textbox", { name: "Relative file path" }), "value.txt");
    await user.type(screen.getByRole("textbox", { name: "Replacement UTF-8 text" }), "after");
    await user.click(screen.getByRole("button", { name: "Create exact contract" }));

    await waitFor(() =>
      expect(mockedDraft).toHaveBeenCalledWith({
        goal: "Replace the value",
        workspace: "/tmp/repository",
        relativePath: "value.txt",
        replacement: "after",
      }),
    );
    expect(screen.getByRole("textbox", { name: "Local mission contract path" })).toHaveValue(
      "/tmp/nemesis-home/drafts/mission.json",
    );
    expect(screen.getByLabelText("Contract SHA-256 digest")).toHaveTextContent(compiled.contractDigest);
  });

  it("compiles, reviews, and authorizes the exact normalized digests", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Missions" }));
    await user.click(screen.getByRole("button", { name: "Review contract" }));
    await user.type(
      screen.getByRole("textbox", { name: "Local mission contract path" }),
      "/tmp/mission.json",
    );
    await user.click(screen.getByRole("button", { name: "Compile contract" }));

    await waitFor(() => expect(mockedCompile).toHaveBeenCalledWith("/tmp/mission.json"));
    expect(screen.getByLabelText("Contract SHA-256 digest")).toHaveTextContent(compiled.contractDigest);
    expect(screen.getByLabelText("Action SHA-256 digest")).toHaveTextContent(compiled.actionDigest);
    await user.click(screen.getByRole("button", { name: "Review authority" }));

    const heading = screen.getByRole("heading", { name: "Authority review" });
    expect(heading).toHaveFocus();
    expect(screen.getByText("NETWORK DENY")).toBeInTheDocument();
    expect(screen.getByText("PUSH DENY")).toBeInTheDocument();
    expect(screen.getByText("SECRETS DENY")).toBeInTheDocument();
    expect(screen.getByText("ONE-SHOT · EXACT ACTION DIGEST")).toBeInTheDocument();
    expect(screen.getByText("PARENT GRANT → ATTENUATED CHILD")).toBeInTheDocument();
    expect(
      screen.getByText("The SPARK kernel consumes the approval exactly once; replay is refused."),
    ).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Authorize and run" }));

    await waitFor(() =>
      expect(mockedRun).toHaveBeenCalledWith(
        "/tmp/mission.json",
        compiled.contractDigest,
        compiled.actionDigest,
      ),
    );
    await waitFor(() => expect(screen.getByText("MISSION VERIFIED")).toBeInTheDocument());
    expect(screen.getByText("TAMPER REJECTED")).toBeInTheDocument();
  });

  it("guides first launch to a real mission composer without a terminal", async () => {
    mockedStatus.mockResolvedValue({ ...system, firstLaunch: true });
    const user = userEvent.setup();
    render(<App />);

    expect(
      await screen.findByRole("heading", { name: "Reach your first witnessed mission" }),
    ).toBeInTheDocument();
    expect(screen.getByText(system.localHome)).toBeInTheDocument();
    const begin = screen.getByRole("button", { name: "Begin first mission" });
    expect(begin).toBeDisabled();

    await user.type(screen.getByRole("textbox", { name: "Workspace path" }), "/tmp/repository");
    expect(begin).toBeEnabled();
    await user.click(begin);

    expect(await screen.findByRole("heading", { name: "Local missions" })).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Workspace path" })).toHaveValue("/tmp/repository");
  });

  it("keeps first launch fail-closed when a bundled component is not ready", async () => {
    mockedStatus.mockResolvedValue({
      ...system,
      firstLaunch: true,
      ready: false,
      runtime: "MISSING",
    });
    const user = userEvent.setup();
    render(<App />);

    await screen.findByRole("heading", { name: "Reach your first witnessed mission" });
    await user.type(screen.getByRole("textbox", { name: "Workspace path" }), "/tmp/repository");
    expect(screen.getByRole("button", { name: "Begin first mission" })).toBeDisabled();
    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("BLOCKED_FIRST_LAUNCH");
    expect(alert).toHaveTextContent("Runtime: MISSING");
  });

  it("renders typed refusal and concrete recovery guidance", async () => {
    mockedCompile.mockRejectedValueOnce({
      code: "REFUSED_WORKSPACE",
      message: "workspace HEAD does not match baseRevision",
      recovery: "Restore the reviewed clean workspace identity or compile a new contract.",
    });
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Missions" }));
    await user.click(screen.getByRole("button", { name: "Review contract" }));
    await user.type(screen.getByRole("textbox", { name: "Local mission contract path" }), "/tmp/bad.json");
    await user.click(screen.getByRole("button", { name: "Compile contract" }));

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent("REFUSED_WORKSPACE");
    expect(alert).toHaveTextContent("Restore the reviewed clean workspace identity");
  });

  it("loads current authoritative replay and labels model reruns comparative", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Replay" }));

    await waitFor(() => expect(mockedReplay).toHaveBeenCalledTimes(1));
    expect(screen.getByText("Exact state reconstruction")).toBeInTheDocument();
    expect(screen.getByText("Model re-execution is comparative")).toBeInTheDocument();
    expect(screen.getByText("EVENT 0002")).toBeInTheDocument();
  });

  it("persists validated reversible settings", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Settings" }));
    await user.selectOptions(screen.getByRole("combobox", { name: "Text scale" }), "large");
    await user.click(screen.getByRole("checkbox", { name: /Reduce motion/ }));
    expect(screen.queryByRole("checkbox", { name: /Retain completed lanes/ })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Save settings" }));

    await waitFor(() =>
      expect(mockedUpdateSettings).toHaveBeenCalledWith({
        textScale: "large",
        reduceMotion: false,
        // No inert retention control is sent.
      }),
    );
  });

  it("reflects reduced-motion and text-scale on the document element", async () => {
    render(<App />);
    await screen.findByRole("button", { name: "Home" });
    await waitFor(() =>
      expect(document.documentElement.dataset.reduceMotion).toBe("true"),
    );
    expect(document.documentElement.dataset.textScale).toBe("standard");
  });

  it("conveys system health with text, not color alone", async () => {
    render(<App />);
    await screen.findByRole("button", { name: "Home" });
    const health = await screen.findByLabelText("Local system health");
    // The colored dot is decorative; the load-bearing status is textual.
    expect(health.querySelector(".health-dot")).toHaveAttribute("aria-hidden", "true");
    await waitFor(() => expect(screen.getByText("CORE READY")).toBeInTheDocument());
  });

  it("exposes Activity Diff Evidence Conversation on the LOCAL-001 cockpit", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Missions" }));
    expect(screen.getByRole("heading", { name: "Witnessed local change" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Activity" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("heading", { name: "Mission activity" })).toBeInTheDocument();

    await user.click(screen.getByRole("tab", { name: "Diff" }));
    expect(screen.getByText("No compiled contract. Diff is not inferred.")).toBeInTheDocument();

    await user.click(screen.getByRole("tab", { name: "Evidence" }));
    expect(screen.getByRole("heading", { name: "No accepted mission evidence" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "No evidence graph yet" })).toBeInTheDocument();

    await user.click(screen.getByRole("tab", { name: "Conversation" }));
    expect(screen.getByText("Kernel log, not model chat")).toBeInTheDocument();
    expect(screen.queryByText(/Codex said|Claude said|assistant/i)).not.toBeInTheDocument();
  });

  it("keeps every production destination keyboard-reachable with an accessible name", async () => {
    render(<App />);
    await screen.findByRole("button", { name: "Home" });
    const order = [
      "Home",
      "Missions",
      "Workspaces",
      "Agents",
      "Changes",
      "Tests",
      "Knowledge",
      "Skills",
      "Automations",
      "Integrations",
      "Security",
      "Evidence",
      "Replay",
      "Settings",
    ];
    const buttons = order.map((name) => screen.getByRole("button", { name }));
    buttons[0].focus();
    for (let index = 1; index < buttons.length; index += 1) {
      fireEvent.keyDown(buttons[index - 1], { key: "ArrowDown" });
      expect(buttons[index]).toHaveFocus();
    }
  });

  it("renders labeled committed kernel events on the Activity tab", async () => {
    mockedStatus.mockResolvedValue({ ...system, lastMission: result });
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Missions" }));

    expect(screen.getByRole("heading", { name: "Mission activity" })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText("State Transitioned")).toBeInTheDocument());
    expect(screen.getByText("Mission Created")).toBeInTheDocument();
    expect(screen.getByText("Running")).toBeInTheDocument();
    expect(screen.queryByText(/Kind \d+ · State \d+/)).not.toBeInTheDocument();
  });

  it("labels the kernel conversation with kernel enum names, never model chat", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Missions" }));
    await user.click(screen.getByRole("tab", { name: "Conversation" }));

    await waitFor(() =>
      expect(screen.getByText("Mission Created · Contract Compiled")).toBeInTheDocument(),
    );
    expect(screen.getByText("State Transitioned · Running")).toBeInTheDocument();
  });

  it("binds a typed evidence graph alongside the completion court", async () => {
    mockedStatus.mockResolvedValue({ ...system, lastMission: result });
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: "Evidence" }));

    expect(await screen.findByRole("heading", { name: "Evidence graph" })).toBeInTheDocument();
    expect(screen.getByText("DETERMINISTIC CHECK")).toBeInTheDocument();
    expect(screen.getByText("CLAIM · BUILD")).toBeInTheDocument();
    expect(screen.getByText("/usr/bin/git diff --check")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Completion court" })).toBeInTheDocument();
  });
});
