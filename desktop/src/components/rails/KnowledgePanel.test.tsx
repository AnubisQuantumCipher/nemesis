import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { KnowledgePanel } from "./KnowledgePanel";
import { railEntities } from "../../lib/bridge";

vi.mock("../../lib/bridge", () => ({
  railEntities: vi.fn(),
  normalizeFailure: (cause: unknown) => cause,
}));

const fact = {
  id: "fact-runtime",
  relativePath: "knowledge/fact-runtime.json",
  sha256: "a".repeat(64),
  bytes: 280,
  value: {
    schema: "nemesis.rail-knowledge/v1",
    id: "fact-runtime",
    subject: "desktop-runtime",
    predicate: "uses",
    value: "tauri-2",
    scope: "workspace",
    sourceDigest: "b".repeat(64),
    observedSequence: 4,
    status: "proposed",
    supersedes: null,
    contradictedBy: null,
    untrustedExternal: true,
    revision: 1,
  },
};

describe("KnowledgePanel", () => {
  beforeEach(() => {
    vi.mocked(railEntities).mockReset();
  });

  it("lists knowledge entries with provenance metadata", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "knowledge",
      entities: [fact],
      contradictions: [],
    });
    render(<KnowledgePanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(await screen.findByText("fact-runtime")).toBeInTheDocument();
    expect(screen.getByText("desktop-runtime")).toBeInTheDocument();
    expect(screen.getByText("uses")).toBeInTheDocument();
    expect(screen.getByText("tauri-2")).toBeInTheDocument();
    expect(screen.getByText("proposed")).toBeInTheDocument();
    expect(screen.getByText("EXTERNAL UNTRUSTED")).toBeInTheDocument();
    expect(screen.getByText("rev 1")).toBeInTheDocument();
  });

  it("surfaces contradictions above the table instead of resolving them", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "knowledge",
      entities: [fact],
      contradictions: [
        { subject: "desktop-runtime", predicate: "uses", entries: ["fact-runtime", "fact-legacy"] },
      ],
    });
    render(<KnowledgePanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(
      await screen.findByText("CONTRADICTION desktop-runtime uses: fact-runtime vs fact-legacy"),
    ).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("CONTRADICTION");
  });

  it("asserts a new entry with numeric sequence and boolean trust flag", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "knowledge",
      entities: [],
      contradictions: [],
    });
    const onBeginMutation = vi.fn();
    render(<KnowledgePanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    expect(
      await screen.findByText("No knowledge entries. Assert one below."),
    ).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("Entry id"), "fact-db");
    await userEvent.type(screen.getByLabelText("Subject"), "storage");
    await userEvent.type(screen.getByLabelText("Predicate"), "backs");
    await userEvent.type(screen.getByLabelText("Value"), "sqlite");
    await userEvent.selectOptions(screen.getByLabelText("Scope"), "global");
    await userEvent.type(screen.getByLabelText("Source digest (64-hex)"), "c".repeat(64));
    await userEvent.type(screen.getByLabelText("Observed sequence"), "7");
    await userEvent.click(screen.getByLabelText("Untrusted external"));
    await userEvent.click(screen.getByRole("button", { name: "Assert entry" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "knowledge",
      verb: "assert",
      id: "fact-db",
      payload: {
        subject: "storage",
        predicate: "backs",
        value: "sqlite",
        scope: "global",
        sourceDigest: "c".repeat(64),
        observedSequence: 7,
        untrustedExternal: true,
      },
    });
  });

  it("transitions a proposed entry to observed through the governed path", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "knowledge",
      entities: [fact],
      contradictions: [],
    });
    const onBeginMutation = vi.fn();
    render(<KnowledgePanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    await userEvent.selectOptions(
      await screen.findByLabelText("Transition fact-runtime"),
      "observed",
    );
    await userEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "knowledge",
      verb: "transition",
      id: "fact-runtime",
      payload: { toStatus: "observed" },
    });
  });

  it("requires a contradicting entry id when disputing", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "knowledge",
      entities: [fact],
      contradictions: [],
    });
    const onBeginMutation = vi.fn();
    render(<KnowledgePanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    await userEvent.selectOptions(
      await screen.findByLabelText("Transition fact-runtime"),
      "disputed",
    );
    await userEvent.type(
      screen.getByLabelText("Contradicted by fact-runtime"),
      "fact-legacy",
    );
    await userEvent.click(screen.getByRole("button", { name: "Apply" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "knowledge",
      verb: "transition",
      id: "fact-runtime",
      payload: { toStatus: "disputed", contradictedBy: "fact-legacy" },
    });
  });

  it("revokes an entry with an empty payload", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "knowledge",
      entities: [fact],
      contradictions: [],
    });
    const onBeginMutation = vi.fn();
    render(<KnowledgePanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    await userEvent.click(await screen.findByRole("button", { name: "Revoke" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "knowledge",
      verb: "revoke",
      id: "fact-runtime",
      payload: {},
    });
  });

  it("surfaces typed refusals instead of hiding them", async () => {
    vi.mocked(railEntities).mockRejectedValue({
      code: "BLOCKED_RAIL_STATE",
      message: "entity knowledge/x malformed",
      recovery: "Inspect the state repository.",
    });
    render(<KnowledgePanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_RAIL_STATE");
    });
  });

  it("disables mutation controls while a mission is in flight", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "knowledge",
      entities: [fact],
      contradictions: [],
    });
    render(<KnowledgePanel stateVersion={0} busy={true} onBeginMutation={vi.fn()} />);
    expect(await screen.findByRole("button", { name: "Apply" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Revoke" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Assert entry" })).toBeDisabled();
  });
});
