import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { WorkspacesPanel } from "./WorkspacesPanel";
import { railEntities } from "../../lib/bridge";

vi.mock("../../lib/bridge", () => ({
  railEntities: vi.fn(),
  normalizeFailure: (cause: unknown) => cause,
}));

const workspace = {
  id: "nemesis-core",
  relativePath: "workspaces/nemesis-core.json",
  sha256: "a".repeat(64),
  bytes: 280,
  value: {
    schema: "nemesis.rail-workspace/v1",
    id: "nemesis-core",
    name: "Nemesis core",
    canonicalPath: "/repos/nemesis-core",
    status: "active",
    revision: 3,
  },
  derived: {
    reachable: true,
    git: true,
    head: "0123456789abcdef0123",
    clean: true,
  },
};

describe("WorkspacesPanel", () => {
  beforeEach(() => {
    vi.mocked(railEntities).mockReset();
  });

  it("lists governed workspaces with live repository health", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "workspaces",
      entities: [
        workspace,
        {
          ...workspace,
          id: "dead-repo",
          value: {
            ...workspace.value,
            id: "dead-repo",
            name: "Dead repo",
            canonicalPath: "/repos/dead-repo",
          },
          derived: { reachable: false, git: false, head: null, clean: null },
        },
      ],
    });
    render(<WorkspacesPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(await screen.findByText("nemesis-core")).toBeInTheDocument();
    expect(screen.getByText("/repos/nemesis-core")).toBeInTheDocument();
    expect(screen.getAllByText("active")).toHaveLength(2);
    expect(screen.getByText("GIT OK")).toBeInTheDocument();
    expect(screen.getByText("CLEAN")).toBeInTheDocument();
    expect(screen.getByText("head 0123456789ab")).toBeInTheDocument();
    expect(screen.getByText("UNREACHABLE")).toBeInTheDocument();
  });

  it("flags dirty non-git states without inventing health", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "workspaces",
      entities: [
        {
          ...workspace,
          derived: { reachable: true, git: false, head: null, clean: false },
        },
      ],
    });
    render(<WorkspacesPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(await screen.findByText("NOT GIT")).toBeInTheDocument();
    expect(screen.getByText("DIRTY")).toBeInTheDocument();
    expect(screen.queryByText("GIT OK")).not.toBeInTheDocument();
    expect(screen.queryByText("CLEAN")).not.toBeInTheDocument();
  });

  it("registers a workspace through the governed mutation path", async () => {
    vi.mocked(railEntities).mockResolvedValue({ rail: "workspaces", entities: [] });
    const onBeginMutation = vi.fn();
    render(<WorkspacesPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    expect(
      await screen.findByText("No governed workspaces. Register one below."),
    ).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("Workspace id"), "new-workspace");
    await userEvent.type(screen.getByLabelText("Name"), "New workspace");
    await userEvent.type(screen.getByLabelText("Canonical path"), "/repos/new-workspace");
    await userEvent.click(screen.getByRole("button", { name: "Register workspace" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "workspaces",
      verb: "register",
      id: "new-workspace",
      payload: { name: "New workspace", canonicalPath: "/repos/new-workspace" },
    });
  });

  it("revokes a workspace and hides revoke once revoked", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "workspaces",
      entities: [
        workspace,
        {
          ...workspace,
          id: "already-revoked",
          value: {
            ...workspace.value,
            id: "already-revoked",
            status: "revoked",
            revocationReason: "superseded",
          },
        },
      ],
    });
    const onBeginMutation = vi.fn();
    render(<WorkspacesPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    const revokeButtons = await screen.findAllByRole("button", { name: "Revoke" });
    expect(revokeButtons).toHaveLength(1);
    await userEvent.click(revokeButtons[0]);
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "workspaces",
      verb: "revoke",
      id: "nemesis-core",
      payload: {},
    });
  });

  it("surfaces typed refusals instead of hiding them", async () => {
    vi.mocked(railEntities).mockRejectedValue({
      code: "BLOCKED_RAIL_STATE",
      message: "entity workspaces/x malformed",
      recovery: "Inspect the state repository.",
    });
    render(<WorkspacesPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_RAIL_STATE");
    });
  });

  it("disables mutation controls while a mission is in flight", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "workspaces",
      entities: [workspace],
    });
    render(<WorkspacesPanel stateVersion={0} busy={true} onBeginMutation={vi.fn()} />);
    expect(await screen.findByRole("button", { name: "Revoke" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Register workspace" })).toBeDisabled();
  });
});
