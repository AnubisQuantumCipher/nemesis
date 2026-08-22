import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SecurityPanel } from "./SecurityPanel";
import { securitySnapshot } from "../../lib/bridge";

vi.mock("../../lib/bridge", () => ({
  securitySnapshot: vi.fn(),
  normalizeFailure: (cause: unknown) => cause,
}));

const adoptionRecord = {
  schema: "nemesis.adoption-record/v1",
  sequence: 3,
  missionId: "mission-adopt",
  rail: "skills",
  relativePath: "skills/mission-hygiene.json",
  contentDigest: "c".repeat(64),
  contractDigest: "d".repeat(64),
  actionDigest: "e".repeat(64),
  stateCommit: "f".repeat(64),
  previous: "0".repeat(64),
  entryHash: "1".repeat(64),
};

const fullSnapshot = {
  policy: { networkEgress: "DENIED_BY_CONTRACT" },
  secretsPosture: { stored: "NONE" },
  grants: [
    {
      grantId: "grant-7",
      missionId: "mission-grant",
      operations: ["read", "write"],
      scopeDigest: "9".repeat(64),
      maximumBytes: 4096,
      status: "ACTIVE",
    },
  ],
  approvals: [
    {
      approvalId: "approval-1",
      missionId: "mission-armed",
      actionDigest: "a".repeat(64),
      status: "ARMED",
    },
    {
      approvalId: "approval-2",
      missionId: "mission-corrupt",
      actionDigest: "b".repeat(64),
      status: "CORRUPT",
      error: "entry hash mismatch at sequence 2",
    },
  ],
  approvalTally: { armed: 1, consumed: 0, corrupt: 1 },
  adoptionChain: {
    length: 3,
    head: "1".repeat(64),
    verified: true,
    records: [adoptionRecord],
  },
};

const emptySnapshot = {
  policy: {},
  secretsPosture: {},
  grants: [],
  approvals: [],
  approvalTally: { armed: 0, consumed: 0, corrupt: 0 },
  adoptionChain: { length: 0, head: "", verified: false, records: [] },
};

describe("SecurityPanel", () => {
  beforeEach(() => {
    vi.mocked(securitySnapshot).mockReset();
  });

  it("renders all five authority sections with posture tags", async () => {
    vi.mocked(securitySnapshot).mockResolvedValue(fullSnapshot);
    render(<SecurityPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);

    expect(await screen.findByText("POLICY")).toBeInTheDocument();
    expect(screen.getByText("SECRETS")).toBeInTheDocument();
    expect(screen.getByText("APPROVALS")).toBeInTheDocument();
    expect(screen.getByText("GRANTS")).toBeInTheDocument();
    expect(screen.getByText("ADOPTION CHAIN")).toBeInTheDocument();

    const denied = screen.getByText("DENIED_BY_CONTRACT");
    expect(denied).toHaveClass("rail-tag", "is-verified");
    const none = screen.getByText("NONE");
    expect(none).toHaveClass("rail-tag", "is-verified");

    expect(screen.getByText("ARMED 1 / CONSUMED 0 / CORRUPT 1")).toBeInTheDocument();
    expect(screen.getByText("ARMED")).toHaveClass("rail-tag", "is-armed");

    expect(screen.getByText("grant-7")).toBeInTheDocument();
    expect(screen.getByText("read, write")).toBeInTheDocument();
    expect(screen.getByText(`scope ${"9".repeat(12)}…`)).toBeInTheDocument();
    expect(screen.getByText("max 4096 bytes")).toBeInTheDocument();
    expect(screen.getByText("ACTIVE")).toHaveClass("rail-tag", "is-armed");

    expect(
      screen.getByText(`length 3 / head ${"1".repeat(12)}…`)
    ).toBeInTheDocument();
    expect(screen.getByText("VERIFIED")).toHaveClass("rail-tag", "is-verified");
    expect(
      screen.getByText("3 skills skills/mission-hygiene.json")
    ).toBeInTheDocument();
    expect(screen.getByText(`content ${"c".repeat(12)}…`)).toBeInTheDocument();
    expect(screen.getByText(`state ${"f".repeat(12)}…`)).toBeInTheDocument();
  });

  it("shows corrupt approval rows with their error text, never filtered", async () => {
    vi.mocked(securitySnapshot).mockResolvedValue(fullSnapshot);
    render(<SecurityPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);

    expect(await screen.findByText("approval-2")).toBeInTheDocument();
    expect(screen.getByText("CORRUPT")).toHaveClass("rail-tag", "is-corrupt");
    expect(screen.getByText("entry hash mismatch at sequence 2")).toBeInTheDocument();
  });

  it("refetches the snapshot when refreshed", async () => {
    vi.mocked(securitySnapshot).mockResolvedValue(fullSnapshot);
    render(<SecurityPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);

    expect(await screen.findByText("POLICY")).toBeInTheDocument();
    expect(vi.mocked(securitySnapshot)).toHaveBeenCalledTimes(1);

    await userEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() => {
      expect(vi.mocked(securitySnapshot)).toHaveBeenCalledTimes(2);
    });
  });

  it("surfaces typed refusals instead of hiding them", async () => {
    vi.mocked(securitySnapshot).mockRejectedValue({
      code: "BLOCKED_RAIL_STATE",
      message: "security snapshot unreadable",
      recovery: "Inspect the state repository.",
    });
    render(<SecurityPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_RAIL_STATE");
    });
    expect(screen.getByRole("alert")).toHaveTextContent("security snapshot unreadable");
    expect(screen.getByRole("alert")).toHaveTextContent("Inspect the state repository.");
  });

  it("states honest empty postures per section", async () => {
    vi.mocked(securitySnapshot).mockResolvedValue(emptySnapshot);
    render(<SecurityPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);

    expect(await screen.findByText("No policy posture persisted yet.")).toBeInTheDocument();
    expect(screen.getByText("No secrets posture persisted yet.")).toBeInTheDocument();
    expect(screen.getByText("No one-shot approvals persisted yet.")).toBeInTheDocument();
    expect(screen.getByText("No grants persisted yet.")).toBeInTheDocument();
    expect(screen.getByText("No adoption records persisted yet.")).toBeInTheDocument();
    expect(screen.getByText("ARMED 0 / CONSUMED 0 / CORRUPT 0")).toBeInTheDocument();
    expect(screen.getByText("UNVERIFIED")).toHaveClass("rail-tag", "is-corrupt");
  });
});
