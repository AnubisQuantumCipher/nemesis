import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ChangesPanel } from "./ChangesPanel";
import { changesSnapshot } from "../../lib/bridge";

vi.mock("../../lib/bridge", () => ({
  changesSnapshot: vi.fn(),
  normalizeFailure: (cause: unknown) => cause,
}));

const okLane = {
  missionId: "mission-alpha",
  lanePath: "/lanes/mission-alpha",
  status: "OK",
  branch: "lane/mission-alpha",
  head: "abcdef0123456789",
  dirtyCount: 2,
  dirtyFiles: [
    { code: "M", path: "src/main.rs" },
    { code: "A", path: "src/new.rs" },
  ],
  diffShortstat: "2 files changed, 8 insertions(+)",
  workspace: "nemesis",
  relativePath: "lanes/mission-alpha",
};

const cleanLane = {
  missionId: "mission-clean",
  lanePath: "/lanes/mission-clean",
  status: "OK",
  branch: "lane/mission-clean",
  head: "0123456789abcdef",
  dirtyCount: 0,
};

const corruptLane = {
  missionId: "mission-broken",
  lanePath: "/lanes/mission-broken",
  status: "CORRUPT",
  error: "lane head diverged from receipt chain",
};

describe("ChangesPanel", () => {
  beforeEach(() => {
    vi.mocked(changesSnapshot).mockReset();
  });

  it("renders lane metadata with dirty and clean tags", async () => {
    vi.mocked(changesSnapshot).mockResolvedValue({
      laneCount: 2,
      lanes: [okLane, cleanLane],
    });
    render(<ChangesPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(await screen.findByText("mission-alpha")).toBeInTheDocument();
    expect(screen.getByText("lane/mission-alpha")).toBeInTheDocument();
    expect(screen.getByText("abcdef012345")).toBeInTheDocument();
    expect(screen.getByText("nemesis")).toBeInTheDocument();
    expect(screen.getByText("lanes/mission-alpha")).toBeInTheDocument();
    expect(screen.getByText("2 files changed, 8 insertions(+)")).toBeInTheDocument();
    const dirty = screen.getByText("DIRTY 2");
    expect(dirty).toHaveClass("rail-tag", "is-armed");
    expect(screen.getByText("M src/main.rs")).toBeInTheDocument();
    expect(screen.getByText("A src/new.rs")).toBeInTheDocument();
    const clean = screen.getByText("CLEAN");
    expect(clean).toHaveClass("rail-tag", "is-verified");
    expect(screen.getByText("2 lanes tracked")).toBeInTheDocument();
  });

  it("surfaces corrupt lanes with their error, never hiding them", async () => {
    vi.mocked(changesSnapshot).mockResolvedValue({
      laneCount: 1,
      lanes: [corruptLane],
    });
    render(<ChangesPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(await screen.findByText("mission-broken")).toBeInTheDocument();
    const corrupt = screen.getByText("CORRUPT");
    expect(corrupt).toHaveClass("rail-tag", "is-corrupt");
    expect(screen.getByText("lane head diverged from receipt chain")).toBeInTheDocument();
  });

  it("refetches the snapshot when Refresh is clicked", async () => {
    vi.mocked(changesSnapshot).mockResolvedValue({ laneCount: 0, lanes: [] });
    render(<ChangesPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await screen.findByText("No lanes. Missions create isolated lanes on authorization.");
    expect(vi.mocked(changesSnapshot)).toHaveBeenCalledTimes(1);
    await userEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() => {
      expect(vi.mocked(changesSnapshot)).toHaveBeenCalledTimes(2);
    });
  });

  it("surfaces typed refusals instead of hiding them", async () => {
    vi.mocked(changesSnapshot).mockRejectedValue({
      code: "BLOCKED_RAIL_STATE",
      message: "lane index unreadable",
      recovery: "Inspect the state repository.",
    });
    render(<ChangesPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_RAIL_STATE");
    });
    expect(screen.getByRole("alert")).toHaveTextContent("lane index unreadable");
    expect(screen.getByRole("alert")).toHaveTextContent("Inspect the state repository.");
  });

  it("shows an honest empty state when no lanes exist", async () => {
    vi.mocked(changesSnapshot).mockResolvedValue({ laneCount: 0, lanes: [] });
    render(<ChangesPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(
      await screen.findByText("No lanes. Missions create isolated lanes on authorization."),
    ).toBeInTheDocument();
  });
});
