import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { TestsPanel } from "./TestsPanel";
import { railEntities, runRailTest } from "../../lib/bridge";

vi.mock("../../lib/bridge", () => ({
  railEntities: vi.fn(),
  runRailTest: vi.fn(),
  normalizeFailure: (cause: unknown) => cause,
}));

const passingTest = {
  id: "guard-diff",
  relativePath: "tests/guard-diff.json",
  sha256: "a".repeat(64),
  bytes: 280,
  value: {
    schema: "nemesis.rail-test/v1",
    id: "guard-diff",
    name: "Guard diff",
    kind: "git-diff-check",
    workspaceId: "ws-main",
    relativePath: "src/guard.rs",
    expectedSha256: null,
    status: "active",
    revision: 1,
  },
};

const staleTest = {
  id: "hash-pin",
  relativePath: "tests/hash-pin.json",
  sha256: "b".repeat(64),
  bytes: 300,
  value: {
    schema: "nemesis.rail-test/v1",
    id: "hash-pin",
    name: "Hash pin",
    kind: "content-match",
    workspaceId: "ws-main",
    relativePath: "src/pin.rs",
    expectedSha256: "c".repeat(64),
    status: "active",
    revision: 2,
  },
};

const latestRuns = [
  {
    testId: "guard-diff",
    verdict: "PASS",
    sequence: 4,
    recordedUnixSeconds: 1755800000,
    sourceDigest: "d".repeat(64),
    stale: false,
  },
  {
    testId: "hash-pin",
    verdict: "FAIL",
    sequence: 5,
    recordedUnixSeconds: 1755800100,
    sourceDigest: "e".repeat(64),
    stale: true,
  },
];

describe("TestsPanel", () => {
  beforeEach(() => {
    vi.mocked(railEntities).mockReset();
    vi.mocked(runRailTest).mockReset();
  });

  it("lists tests with latest run verdicts and stale marking", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "tests",
      entities: [passingTest, staleTest],
      latestRuns,
    });
    render(<TestsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(await screen.findByText("guard-diff")).toBeInTheDocument();
    expect(screen.getByText("hash-pin")).toBeInTheDocument();
    const table = within(screen.getByRole("list", { name: "Governed tests" }));
    expect(table.getByText("git-diff-check")).toBeInTheDocument();
    expect(screen.getByText("src/guard.rs")).toBeInTheDocument();
    expect(screen.getByText("PASS")).toBeInTheDocument();
    expect(screen.getByText("FAIL")).toBeInTheDocument();
    expect(screen.getByText("STALE")).toBeInTheDocument();
  });

  it("registers a git-diff-check test without an expectedSha256 key", async () => {
    vi.mocked(railEntities).mockResolvedValue({ rail: "tests", entities: [] });
    const onBeginMutation = vi.fn();
    render(<TestsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    expect(
      await screen.findByText("No governed tests. Register one below."),
    ).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("Test id"), "diff-guard");
    await userEvent.type(screen.getByLabelText("Name"), "Diff guard");
    await userEvent.type(screen.getByLabelText("Workspace id"), "ws-main");
    await userEvent.type(screen.getByLabelText("Relative path"), "src/main.rs");
    await userEvent.click(screen.getByRole("button", { name: "Register test" }));
    expect(onBeginMutation).toHaveBeenCalledTimes(1);
    const request = onBeginMutation.mock.calls[0][0];
    expect(request).toEqual({
      rail: "tests",
      verb: "register",
      id: "diff-guard",
      payload: {
        name: "Diff guard",
        kind: "git-diff-check",
        workspaceId: "ws-main",
        relativePath: "src/main.rs",
      },
    });
    expect("expectedSha256" in request.payload).toBe(false);
  });

  it("registers a content-match test carrying the expected digest", async () => {
    vi.mocked(railEntities).mockResolvedValue({ rail: "tests", entities: [] });
    const onBeginMutation = vi.fn();
    render(<TestsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    await screen.findByText("No governed tests. Register one below.");
    await userEvent.type(screen.getByLabelText("Test id"), "pin-check");
    await userEvent.type(screen.getByLabelText("Name"), "Pin check");
    await userEvent.selectOptions(screen.getByLabelText("Kind"), "content-match");
    await userEvent.type(screen.getByLabelText("Workspace id"), "ws-main");
    await userEvent.type(screen.getByLabelText("Relative path"), "src/pin.rs");
    await userEvent.type(
      screen.getByLabelText("Expected SHA-256 (content-match only)"),
      "f".repeat(64),
    );
    await userEvent.click(screen.getByRole("button", { name: "Register test" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "tests",
      verb: "register",
      id: "pin-check",
      payload: {
        name: "Pin check",
        kind: "content-match",
        workspaceId: "ws-main",
        relativePath: "src/pin.rs",
        expectedSha256: "f".repeat(64),
      },
    });
  });

  it("runs a test directly, refetches, and shows the receipt verdict inline", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "tests",
      entities: [passingTest],
      latestRuns: [latestRuns[0]],
    });
    vi.mocked(runRailTest).mockResolvedValue({
      schema: "nemesis.receipt/v1",
      sequence: 6,
      subject: "tests/guard-diff",
      verdict: "PASS",
      detail: {},
      recordedUnixSeconds: 1755800200,
      previous: "0".repeat(64),
      entryHash: "1".repeat(64),
    });
    render(<TestsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Run" }));
    await waitFor(() => {
      expect(runRailTest).toHaveBeenCalledWith("guard-diff");
    });
    await waitFor(() => {
      expect(railEntities).toHaveBeenCalledTimes(2);
    });
    expect(await screen.findByText("RECEIPT PASS")).toBeInTheDocument();
  });

  it("surfaces a run refusal through the typed alert", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "tests",
      entities: [passingTest],
      latestRuns: [],
    });
    vi.mocked(runRailTest).mockRejectedValue({
      code: "BLOCKED_STALE_SOURCE",
      message: "test guard-diff cannot run",
      recovery: "Re-pin the test to current source.",
    });
    render(<TestsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Run" }));
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_STALE_SOURCE");
    });
    expect(screen.getByRole("alert")).toHaveTextContent("Re-pin the test to current source.");
  });

  it("revokes through the governed mutation path", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "tests",
      entities: [passingTest],
      latestRuns: [],
    });
    const onBeginMutation = vi.fn();
    render(<TestsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    await userEvent.click(await screen.findByRole("button", { name: "Revoke" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "tests",
      verb: "revoke",
      id: "guard-diff",
      payload: {},
    });
  });

  it("surfaces typed refusals from the listing instead of hiding them", async () => {
    vi.mocked(railEntities).mockRejectedValue({
      code: "BLOCKED_RAIL_STATE",
      message: "entity tests/x malformed",
      recovery: "Inspect the state repository.",
    });
    render(<TestsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_RAIL_STATE");
    });
  });

  it("disables mutation controls while a mission is in flight", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "tests",
      entities: [passingTest],
      latestRuns: [],
    });
    render(<TestsPanel stateVersion={0} busy={true} onBeginMutation={vi.fn()} />);
    expect(await screen.findByRole("button", { name: "Revoke" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Register test" })).toBeDisabled();
  });
});
