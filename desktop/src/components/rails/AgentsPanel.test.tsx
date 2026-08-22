import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AgentsPanel } from "./AgentsPanel";
import { railEntities } from "../../lib/bridge";

vi.mock("../../lib/bridge", () => ({
  railEntities: vi.fn(),
  normalizeFailure: (cause: unknown) => cause,
}));

const agent = {
  id: "planner-alpha",
  relativePath: "agents/planner-alpha.json",
  sha256: "a".repeat(64),
  bytes: 420,
  value: {
    schema: "nemesis.rail-agent/v1",
    id: "planner-alpha",
    name: "Planner Alpha",
    providerKind: "codex-cli",
    executablePath: "/usr/local/bin/codex",
    roles: ["planner", "reviewer"],
    consequenceCeiling: "advisory",
    budget: { costMicrounits: 1000, inputTokens: 2048, outputTokens: 1024 },
    status: "active",
    revision: 3,
  },
  derived: { availability: "READY" },
};

const deniedAgent = {
  ...agent,
  id: "api-worker",
  relativePath: "agents/api-worker.json",
  value: {
    ...agent.value,
    id: "api-worker",
    name: "API Worker",
    providerKind: "openai-api",
    executablePath: null,
  },
  derived: { availability: "UNAVAILABLE_NETWORK_DENIED" },
};

describe("AgentsPanel", () => {
  beforeEach(() => {
    vi.mocked(railEntities).mockReset();
  });

  it("lists governed agents with roles, ceiling, and availability tags", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "agents",
      entities: [agent, deniedAgent],
    });
    render(<AgentsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(await screen.findByText("planner-alpha")).toBeInTheDocument();
    expect(screen.getAllByText("planner, reviewer")).toHaveLength(2);
    expect(screen.getAllByText("advisory")).not.toHaveLength(0);
    const ready = screen.getByText("READY");
    expect(ready).toHaveClass("rail-tag", "is-verified");
    const denied = screen.getByText("UNAVAILABLE_NETWORK_DENIED");
    expect(denied).toHaveClass("rail-tag", "is-armed");
    expect(screen.getAllByText("active")).toHaveLength(2);
  });

  it("marks a missing executable as corrupt availability", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "agents",
      entities: [
        {
          ...agent,
          derived: { availability: "MISSING_EXECUTABLE" },
        },
      ],
    });
    render(<AgentsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    const missing = await screen.findByText("MISSING_EXECUTABLE");
    expect(missing).toHaveClass("rail-tag", "is-corrupt");
  });

  it("registers an agent with roles array and numeric budget", async () => {
    vi.mocked(railEntities).mockResolvedValue({ rail: "agents", entities: [] });
    const onBeginMutation = vi.fn();
    render(<AgentsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    expect(
      await screen.findByText("No governed agents. Register one below."),
    ).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("Agent id"), "builder-beta");
    await userEvent.type(screen.getByLabelText("Name"), "Builder Beta");
    await userEvent.selectOptions(screen.getByLabelText("Provider kind"), "local-model");
    await userEvent.click(screen.getByLabelText("builder"));
    await userEvent.click(screen.getByLabelText("verifier"));
    await userEvent.selectOptions(
      screen.getByLabelText("Consequence ceiling"),
      "decision-boundary",
    );
    const cost = screen.getByLabelText("Budget cost microunits");
    await userEvent.clear(cost);
    await userEvent.type(cost, "5000");
    const input = screen.getByLabelText("Budget input tokens");
    await userEvent.clear(input);
    await userEvent.type(input, "4096");
    const output = screen.getByLabelText("Budget output tokens");
    await userEvent.clear(output);
    await userEvent.type(output, "2048");
    await userEvent.click(screen.getByRole("button", { name: "Register agent" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "agents",
      verb: "register",
      id: "builder-beta",
      payload: {
        name: "Builder Beta",
        providerKind: "local-model",
        roles: ["builder", "verifier"],
        consequenceCeiling: "decision-boundary",
        budget: { costMicrounits: 5000, inputTokens: 4096, outputTokens: 2048 },
      },
    });
  });

  it("includes the executable path only when provided", async () => {
    vi.mocked(railEntities).mockResolvedValue({ rail: "agents", entities: [] });
    const onBeginMutation = vi.fn();
    render(<AgentsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    await screen.findByText("No governed agents. Register one below.");
    await userEvent.type(screen.getByLabelText("Agent id"), "sub-proc");
    await userEvent.type(screen.getByLabelText("Name"), "Sub Proc");
    await userEvent.type(screen.getByLabelText("Executable path"), "/usr/local/bin/agent");
    await userEvent.click(screen.getByRole("button", { name: "Register agent" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "agents",
      verb: "register",
      id: "sub-proc",
      payload: {
        name: "Sub Proc",
        providerKind: "generic-subprocess",
        executablePath: "/usr/local/bin/agent",
        roles: [],
        consequenceCeiling: "informational",
        budget: { costMicrounits: 0, inputTokens: 0, outputTokens: 0 },
      },
    });
  });

  it("revokes an agent through the governed mutation path", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "agents",
      entities: [agent],
    });
    const onBeginMutation = vi.fn();
    render(<AgentsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    await userEvent.click(await screen.findByRole("button", { name: "Revoke" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "agents",
      verb: "revoke",
      id: "planner-alpha",
      payload: {},
    });
  });

  it("surfaces typed refusals instead of hiding them", async () => {
    vi.mocked(railEntities).mockRejectedValue({
      code: "BLOCKED_RAIL_STATE",
      message: "entity agents/x malformed",
      recovery: "Inspect the state repository.",
    });
    render(<AgentsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_RAIL_STATE");
    });
  });

  it("disables mutation controls while a mission is in flight", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "agents",
      entities: [agent],
    });
    render(<AgentsPanel stateVersion={0} busy={true} onBeginMutation={vi.fn()} />);
    expect(await screen.findByRole("button", { name: "Revoke" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Register agent" })).toBeDisabled();
  });
});
