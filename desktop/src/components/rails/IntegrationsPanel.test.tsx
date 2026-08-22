import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { IntegrationsPanel } from "./IntegrationsPanel";
import { executeIntegrationPlugin, railEntities } from "../../lib/bridge";

vi.mock("../../lib/bridge", () => ({
  railEntities: vi.fn(),
  executeIntegrationPlugin: vi.fn(),
  normalizeFailure: (cause: unknown) => cause,
}));

const wasmPlugin = {
  id: "digest-tool",
  relativePath: "integrations/digest-tool.json",
  sha256: "a".repeat(64),
  bytes: 480,
  value: {
    schema: "nemesis.rail-integration/v1",
    id: "digest-tool",
    name: "Digest tool",
    kind: "wasm-plugin",
    moduleDigest: "c".repeat(64),
    export: "run",
    tools: [{ name: "digest", schemaDigest: "d".repeat(64) }],
    capabilities: { networkHosts: [], filesystemRoots: [], secrets: [], events: [] },
    status: "enabled",
    revision: 2,
  },
  derived: {},
};

const externalProvider = {
  id: "codex-bridge",
  relativePath: "integrations/codex-bridge.json",
  sha256: "b".repeat(64),
  bytes: 260,
  value: {
    schema: "nemesis.rail-integration/v1",
    id: "codex-bridge",
    name: "Codex bridge",
    kind: "external-provider",
    providerKind: "codex-cli",
    tools: [{ name: "draft", schemaDigest: "e".repeat(64) }],
    capabilities: { networkHosts: [], filesystemRoots: [], secrets: [], events: [] },
    status: "proposed",
    revision: 1,
  },
  derived: {},
};

describe("IntegrationsPanel", () => {
  beforeEach(() => {
    vi.mocked(railEntities).mockReset();
    vi.mocked(executeIntegrationPlugin).mockReset();
  });

  it("lists integrations with tools, module digest, deny tags, and status", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "integrations",
      entities: [wasmPlugin, externalProvider],
    });
    render(<IntegrationsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(await screen.findByText("digest-tool")).toBeInTheDocument();
    expect(screen.getByText("Digest tool")).toBeInTheDocument();
    const list = screen.getByRole("list", { name: "Governed integrations" });
    expect(within(list).getByText("wasm-plugin")).toBeInTheDocument();
    expect(screen.getByText(`tool digest ${"d".repeat(8)}`)).toBeInTheDocument();
    expect(screen.getByText(`module ${"c".repeat(12)}…`)).toBeInTheDocument();
    expect(screen.getAllByText("NET DENY / FS DENY / SECRETS DENY")).toHaveLength(2);
    expect(screen.getByText("enabled")).toBeInTheDocument();
    expect(screen.getByText("codex-cli")).toBeInTheDocument();
    expect(screen.getByText("proposed")).toBeInTheDocument();
  });

  it("registers a wasm plugin with module WAT, export, and one frozen tool", async () => {
    vi.mocked(railEntities).mockResolvedValue({ rail: "integrations", entities: [] });
    const onBeginMutation = vi.fn();
    render(
      <IntegrationsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />,
    );
    expect(
      await screen.findByText("No governed integrations. Register one below."),
    ).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("Integration id"), "adder");
    await userEvent.type(screen.getByLabelText("Name"), "Adder");
    await userEvent.selectOptions(screen.getByLabelText("Kind"), "wasm-plugin");
    await userEvent.type(screen.getByLabelText("Module WAT"), "(module)");
    await userEvent.type(screen.getByLabelText("Export"), "add");
    await userEvent.type(screen.getByLabelText("Tool name"), "add");
    await userEvent.type(
      screen.getByLabelText("Tool schema digest (64-hex)"),
      "f".repeat(64),
    );
    await userEvent.click(screen.getByRole("button", { name: "Register integration" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "integrations",
      verb: "register",
      id: "adder",
      payload: {
        name: "Adder",
        kind: "wasm-plugin",
        tools: [{ name: "add", schemaDigest: "f".repeat(64) }],
        moduleWat: "(module)",
        export: "add",
      },
    });
  });

  it("registers an external provider with a provider kind and no capabilities key", async () => {
    vi.mocked(railEntities).mockResolvedValue({ rail: "integrations", entities: [] });
    const onBeginMutation = vi.fn();
    render(
      <IntegrationsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />,
    );
    await screen.findByText("No governed integrations. Register one below.");
    await userEvent.type(screen.getByLabelText("Integration id"), "claude-lane");
    await userEvent.type(screen.getByLabelText("Name"), "Claude lane");
    await userEvent.selectOptions(screen.getByLabelText("Kind"), "external-provider");
    await userEvent.selectOptions(
      screen.getByLabelText("Provider kind"),
      "claude-code-cli",
    );
    await userEvent.type(screen.getByLabelText("Tool name"), "draft");
    await userEvent.type(
      screen.getByLabelText("Tool schema digest (64-hex)"),
      "e".repeat(64),
    );
    await userEvent.click(screen.getByRole("button", { name: "Register integration" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "integrations",
      verb: "register",
      id: "claude-lane",
      payload: {
        name: "Claude lane",
        kind: "external-provider",
        providerKind: "claude-code-cli",
        tools: [{ name: "draft", schemaDigest: "e".repeat(64) }],
      },
    });
    expect(onBeginMutation.mock.calls[0][0].payload).not.toHaveProperty("capabilities");
  });

  it("enables proposed and disables enabled integrations through governed mutations", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "integrations",
      entities: [wasmPlugin, externalProvider],
    });
    const onBeginMutation = vi.fn();
    render(
      <IntegrationsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />,
    );
    await userEvent.click(await screen.findByRole("button", { name: "Enable" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "integrations",
      verb: "enable",
      id: "codex-bridge",
      payload: {},
    });
    await userEvent.click(screen.getByRole("button", { name: "Disable" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "integrations",
      verb: "disable",
      id: "digest-tool",
      payload: {},
    });
  });

  it("executes an enabled wasm plugin and renders the chained receipt verdict", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "integrations",
      entities: [wasmPlugin],
    });
    vi.mocked(executeIntegrationPlugin).mockResolvedValue({
      schema: "nemesis.receipt/v1",
      sequence: 7,
      subject: "integrations/digest-tool",
      verdict: "EXECUTED",
      detail: { result: "42" },
      recordedUnixSeconds: 1_755_800_000,
      previous: "0".repeat(64),
      entryHash: "1".repeat(64),
    });
    render(<IntegrationsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Execute" }));
    expect(executeIntegrationPlugin).toHaveBeenCalledWith("digest-tool");
    expect(await screen.findByText("EXECUTED")).toBeInTheDocument();
    expect(screen.getByText("42")).toBeInTheDocument();
  });

  it("surfaces an execution refusal as a REFUSED verdict tag", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "integrations",
      entities: [wasmPlugin],
    });
    vi.mocked(executeIntegrationPlugin).mockResolvedValue({
      schema: "nemesis.receipt/v1",
      sequence: 8,
      subject: "integrations/digest-tool",
      verdict: "REFUSED",
      detail: {},
      recordedUnixSeconds: 1_755_800_001,
      previous: "1".repeat(64),
      entryHash: "2".repeat(64),
    });
    render(<IntegrationsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: "Execute" }));
    expect(await screen.findByText("REFUSED")).toBeInTheDocument();
  });

  it("revokes a non-revoked integration through the governed mutation path", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "integrations",
      entities: [externalProvider],
    });
    const onBeginMutation = vi.fn();
    render(
      <IntegrationsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />,
    );
    await userEvent.click(await screen.findByRole("button", { name: "Revoke" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "integrations",
      verb: "revoke",
      id: "codex-bridge",
      payload: {},
    });
  });

  it("surfaces typed refusals instead of hiding them", async () => {
    vi.mocked(railEntities).mockRejectedValue({
      code: "BLOCKED_RAIL_STATE",
      message: "entity integrations/x malformed",
      recovery: "Inspect the state repository.",
    });
    render(<IntegrationsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_RAIL_STATE");
    });
  });

  it("disables mutation controls while a mission is in flight", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "integrations",
      entities: [wasmPlugin, externalProvider],
    });
    render(<IntegrationsPanel stateVersion={0} busy={true} onBeginMutation={vi.fn()} />);
    expect(await screen.findByRole("button", { name: "Enable" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Disable" })).toBeDisabled();
    for (const revoke of screen.getAllByRole("button", { name: "Revoke" })) {
      expect(revoke).toBeDisabled();
    }
    expect(screen.getByRole("button", { name: "Register integration" })).toBeDisabled();
  });
});
