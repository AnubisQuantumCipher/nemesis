import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AutomationsPanel } from "./AutomationsPanel";
import { dispatchAutomation, railEntities } from "../../lib/bridge";

vi.mock("../../lib/bridge", () => ({
  railEntities: vi.fn(),
  dispatchAutomation: vi.fn(),
  normalizeFailure: (cause: unknown) => cause,
}));

const automation = {
  id: "nightly-digest",
  relativePath: "automations/nightly-digest.json",
  sha256: "a".repeat(64),
  bytes: 512,
  value: {
    schema: "nemesis.rail-automation/v1",
    id: "nightly-digest",
    name: "Nightly digest",
    operation: "draft-mission",
    trigger: { kind: "local-schedule", key: "nightly", dueUnixSeconds: 1766000000 },
    missionTemplate: {
      goal: "Summarize yesterday's receipts into a governed digest for review",
      workspaceId: "ws-main",
      relativePath: "digests/latest.md",
      replacement: "# digest",
    },
    status: "queued",
    revision: 3,
  },
  derived: {},
};

describe("AutomationsPanel", () => {
  beforeEach(() => {
    vi.mocked(railEntities).mockReset();
    vi.mocked(dispatchAutomation).mockReset();
  });

  it("lists governed automations with trigger, template, and status", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "automations",
      entities: [automation],
    });
    render(<AutomationsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(await screen.findByText("nightly-digest")).toBeInTheDocument();
    expect(screen.getByText("Nightly digest")).toBeInTheDocument();
    expect(screen.getByText("local-schedule nightly")).toBeInTheDocument();
    expect(screen.getByText("due 1766000000")).toBeInTheDocument();
    expect(
      screen.getByText("Summarize yesterday's receipts into a governed d…"),
    ).toBeInTheDocument();
    expect(screen.getByText("ws-main")).toBeInTheDocument();
    expect(screen.getByText("queued")).toHaveClass("is-armed");
    expect(screen.getByText("rev 3")).toBeInTheDocument();
  });

  it("enqueues a manual automation omitting dueUnixSeconds", async () => {
    vi.mocked(railEntities).mockResolvedValue({ rail: "automations", entities: [] });
    const onBeginMutation = vi.fn();
    render(<AutomationsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    expect(
      await screen.findByText("No governed automations. Enqueue one below."),
    ).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("Automation id"), "manual-draft");
    await userEvent.type(screen.getByLabelText("Name"), "Manual draft");
    await userEvent.selectOptions(screen.getByLabelText("Trigger kind"), "manual");
    await userEvent.type(screen.getByLabelText("Trigger key"), "on-demand");
    await userEvent.type(screen.getByLabelText("Due unix seconds"), "1766000000");
    await userEvent.type(screen.getByLabelText("Goal"), "Draft a mission");
    await userEvent.type(screen.getByLabelText("Workspace id"), "ws-main");
    await userEvent.type(screen.getByLabelText("Relative path"), "notes/plan.md");
    await userEvent.type(screen.getByLabelText("Replacement"), "# plan");
    await userEvent.click(screen.getByRole("button", { name: "Enqueue automation" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "automations",
      verb: "enqueue",
      id: "manual-draft",
      payload: {
        name: "Manual draft",
        operation: "draft-mission",
        trigger: { kind: "manual", key: "on-demand" },
        missionTemplate: {
          goal: "Draft a mission",
          workspaceId: "ws-main",
          relativePath: "notes/plan.md",
          replacement: "# plan",
        },
      },
    });
  });

  it("enqueues a local-schedule automation including dueUnixSeconds", async () => {
    vi.mocked(railEntities).mockResolvedValue({ rail: "automations", entities: [] });
    const onBeginMutation = vi.fn();
    render(<AutomationsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    await screen.findByText("No governed automations. Enqueue one below.");
    await userEvent.type(screen.getByLabelText("Automation id"), "scheduled-draft");
    await userEvent.type(screen.getByLabelText("Name"), "Scheduled draft");
    await userEvent.selectOptions(screen.getByLabelText("Trigger kind"), "local-schedule");
    await userEvent.type(screen.getByLabelText("Trigger key"), "nightly");
    await userEvent.type(screen.getByLabelText("Due unix seconds"), "1766000000");
    await userEvent.type(screen.getByLabelText("Goal"), "Draft the digest");
    await userEvent.type(screen.getByLabelText("Workspace id"), "ws-main");
    await userEvent.type(screen.getByLabelText("Relative path"), "digests/latest.md");
    await userEvent.type(screen.getByLabelText("Replacement"), "# digest");
    await userEvent.click(screen.getByRole("button", { name: "Enqueue automation" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "automations",
      verb: "enqueue",
      id: "scheduled-draft",
      payload: {
        name: "Scheduled draft",
        operation: "draft-mission",
        trigger: { kind: "local-schedule", key: "nightly", dueUnixSeconds: 1766000000 },
        missionTemplate: {
          goal: "Draft the digest",
          workspaceId: "ws-main",
          relativePath: "digests/latest.md",
          replacement: "# digest",
        },
      },
    });
  });

  it("dispatches a queued automation and renders the authorized receipt verdict", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "automations",
      entities: [automation],
    });
    vi.mocked(dispatchAutomation).mockResolvedValue({
      schema: "nemesis.receipt/v1",
      sequence: 7,
      subject: "automations/nightly-digest",
      verdict: "AUTHORIZED_DRAFT_ONLY",
      detail: { boundary: "draft parked for human review" },
      recordedUnixSeconds: 1766000001,
      previous: "0".repeat(64),
      entryHash: "f".repeat(64),
    });
    render(<AutomationsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Dispatch" }));
    expect(dispatchAutomation).toHaveBeenCalledWith("nightly-digest");
    expect(await screen.findByText("AUTHORIZED_DRAFT_ONLY")).toHaveClass("is-verified");
    expect(screen.getByText("draft parked for human review")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("renders a DENIED dispatch verdict as an armed refusal, not an error", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "automations",
      entities: [automation],
    });
    vi.mocked(dispatchAutomation).mockResolvedValue({
      schema: "nemesis.receipt/v1",
      sequence: 8,
      subject: "automations/nightly-digest",
      verdict: "DENIED_AUTHORITY_BOUNDARY",
      detail: { boundary: "dispatch refused: run remains a human act" },
      recordedUnixSeconds: 1766000002,
      previous: "0".repeat(64),
      entryHash: "e".repeat(64),
    });
    render(<AutomationsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Dispatch" }));
    expect(await screen.findByText("DENIED_AUTHORITY_BOUNDARY")).toHaveClass("is-armed");
    expect(screen.getByText("dispatch refused: run remains a human act")).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("surfaces dispatch failures as typed panel errors", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "automations",
      entities: [automation],
    });
    vi.mocked(dispatchAutomation).mockRejectedValue({
      code: "BLOCKED_DISPATCH",
      message: "receipt chain unavailable",
      recovery: "Inspect the receipt ledger.",
    });
    render(<AutomationsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Dispatch" }));
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_DISPATCH");
    });
  });

  it("revokes through the governed mutation path", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "automations",
      entities: [automation],
    });
    const onBeginMutation = vi.fn();
    render(<AutomationsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    await userEvent.click(await screen.findByRole("button", { name: "Revoke" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "automations",
      verb: "revoke",
      id: "nightly-digest",
      payload: {},
    });
  });

  it("surfaces typed refusals instead of hiding them", async () => {
    vi.mocked(railEntities).mockRejectedValue({
      code: "BLOCKED_RAIL_STATE",
      message: "entity automations/x malformed",
      recovery: "Inspect the state repository.",
    });
    render(<AutomationsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_RAIL_STATE");
    });
  });

  it("disables mutation controls while a mission is in flight", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "automations",
      entities: [automation],
    });
    render(<AutomationsPanel stateVersion={0} busy={true} onBeginMutation={vi.fn()} />);
    expect(await screen.findByRole("button", { name: "Dispatch" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Revoke" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Enqueue automation" })).toBeDisabled();
  });
});
