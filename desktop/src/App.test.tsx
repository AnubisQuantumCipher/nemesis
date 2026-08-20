import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";
import { getSystemStatus, runWitnessedMission } from "./lib/bridge";

vi.mock("./lib/bridge", () => ({
  getSystemStatus: vi.fn(),
  runWitnessedMission: vi.fn(),
}));

const mockedStatus = vi.mocked(getSystemStatus);
const mockedRun = vi.mocked(runWitnessedMission);

beforeEach(() => {
  mockedStatus.mockResolvedValue({
    ready: true,
    core: "READY",
    kernel: "PROVED_LOCAL",
    runtime: "READY",
    contractSha256:
      "ebebe1a7b3fc11458d21ee9eedd1b724ad08b38d84372733e0d8beebee532779",
  });
  mockedRun.mockResolvedValue({
    status: "VERIFIED",
    missionId: "mis_0000000000000000000000",
    sequence: 12,
    sourceDigest: "44".repeat(32),
    ledgerHead: "55".repeat(32),
    artifactDirectory: "/tmp/nemesis-evidence",
    claims: [
      { id: "build", status: "VERIFIED", evidenceDigest: "66".repeat(32) },
      { id: "tests", status: "VERIFIED", evidenceDigest: "77".repeat(32) },
    ],
    tamperVerdict: "REJECTED",
  });
});

describe("NEMESIS Desktop", () => {
  it("locks the product identity and exposes the complete desktop navigation", async () => {
    render(<App />);

    expect(screen.getByRole("heading", { name: "NEMESIS" })).toBeInTheDocument();
    expect(screen.getByText("The Provable Agent Operating System")).toBeInTheDocument();
    expect(
      screen.getByText("Intelligence proposes. NEMESIS governs. Evidence decides."),
    ).toBeInTheDocument();

    for (const name of [
      "Home",
      "Missions",
      "Workspaces",
      "Agents",
      "Changes",
      "Tests",
      "Evidence",
      "Knowledge",
      "Skills",
      "Automations",
      "Integrations",
      "Security",
      "Replay",
      "Settings",
    ]) {
      expect(screen.getByRole("button", { name })).toBeInTheDocument();
    }
    await waitFor(() => expect(screen.getByText("CORE READY")).toBeInTheDocument());
  });

  it("reviews exact authority before running the witnessed mission", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("button", { name: "Review contract" }));
    expect(screen.getByRole("heading", { name: "Authority review" })).toBeInTheDocument();
    expect(screen.getByText("Push disabled")).toBeInTheDocument();
    expect(screen.getByText("Publish disabled")).toBeInTheDocument();
    expect(screen.getByText("Worker cannot mark complete")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Authorize and run" }));
    expect(mockedRun).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.getByText("MISSION VERIFIED")).toBeInTheDocument());
    expect(screen.getAllByText("VERIFIED").length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("TAMPER REJECTED")).toBeInTheDocument();
    expect(screen.getByText("44".repeat(32))).toBeInTheDocument();
  });

  it("keeps epistemic statuses textual and exposes the completion court", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(screen.getByLabelText("Evidence status VERIFIED")).toBeInTheDocument();
    expect(screen.getByLabelText("Evidence status BELIEVED")).toBeInTheDocument();
    expect(screen.getByLabelText("Evidence status UNKNOWN")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Evidence" }));
    expect(screen.getByRole("heading", { name: "Completion court" })).toBeInTheDocument();
    expect(screen.getByText("Final source binding")).toBeInTheDocument();
    expect(screen.getByText("Kernel decision")).toBeInTheDocument();
  });
});
