import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { SkillsPanel } from "./SkillsPanel";
import { railEntities } from "../../lib/bridge";

vi.mock("../../lib/bridge", () => ({
  railEntities: vi.fn(),
  normalizeFailure: (cause: unknown) => cause,
}));

const skill = {
  id: "mission-hygiene",
  relativePath: "skills/mission-hygiene.json",
  sha256: "a".repeat(64),
  bytes: 320,
  value: {
    schema: "nemesis.rail-skill/v1",
    id: "mission-hygiene",
    name: "Mission hygiene",
    bodyDigest: "b".repeat(64),
    bodyBytes: 512,
    status: "proposed",
    revision: 1,
  },
  derived: { bodyPresent: true, bodyVerified: true },
};

describe("SkillsPanel", () => {
  beforeEach(() => {
    vi.mocked(railEntities).mockReset();
  });

  it("lists governed skills with trust state and body integrity", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "skills",
      entities: [skill],
    });
    render(<SkillsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    expect(await screen.findByText("mission-hygiene")).toBeInTheDocument();
    expect(screen.getByText("proposed")).toBeInTheDocument();
    expect(screen.getByText("BODY VERIFIED")).toBeInTheDocument();
  });

  it("advances one trust state without skipping", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "skills",
      entities: [skill],
    });
    const onBeginMutation = vi.fn();
    render(<SkillsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    await userEvent.click(await screen.findByRole("button", { name: "Advance" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "skills",
      verb: "advance",
      id: "mission-hygiene",
      payload: { toStatus: "staged" },
    });
  });

  it("proposes a new skill through the governed mutation path", async () => {
    vi.mocked(railEntities).mockResolvedValue({ rail: "skills", entities: [] });
    const onBeginMutation = vi.fn();
    render(<SkillsPanel stateVersion={0} busy={false} onBeginMutation={onBeginMutation} />);
    expect(await screen.findByText("No governed skills. Propose one below.")).toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("Skill id"), "new-skill");
    await userEvent.type(screen.getByLabelText("Name"), "New skill");
    await userEvent.type(screen.getByLabelText("Procedure body"), "# body");
    await userEvent.click(screen.getByRole("button", { name: "Propose skill" }));
    expect(onBeginMutation).toHaveBeenCalledWith({
      rail: "skills",
      verb: "propose",
      id: "new-skill",
      payload: { name: "New skill", body: "# body" },
    });
  });

  it("surfaces typed refusals instead of hiding them", async () => {
    vi.mocked(railEntities).mockRejectedValue({
      code: "BLOCKED_RAIL_STATE",
      message: "entity skills/x malformed",
      recovery: "Inspect the state repository.",
    });
    render(<SkillsPanel stateVersion={0} busy={false} onBeginMutation={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("BLOCKED_RAIL_STATE");
    });
  });

  it("disables mutation controls while a mission is in flight", async () => {
    vi.mocked(railEntities).mockResolvedValue({
      rail: "skills",
      entities: [skill],
    });
    render(<SkillsPanel stateVersion={0} busy={true} onBeginMutation={vi.fn()} />);
    expect(await screen.findByRole("button", { name: "Advance" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Revoke" })).toBeDisabled();
  });
});
