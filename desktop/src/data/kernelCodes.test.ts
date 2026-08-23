import { describe, expect, it } from "vitest";

import { eventLabel, stateLabel } from "./kernelCodes";

describe("kernel code labels", () => {
  it("maps state codes to kernel Mission_State names (nemesis-kernel-types.ads)", () => {
    expect(stateLabel(0)).toBe("Draft");
    expect(stateLabel(3)).toBe("Planning");
    expect(stateLabel(4)).toBe("Running");
    expect(stateLabel(7)).toBe("Verifying");
    expect(stateLabel(8)).toBe("Complete");
    expect(stateLabel(9)).toBe("Blocked With Evidence");
    expect(stateLabel(10)).toBe("Cancelled");
  });

  it("maps kind codes to daemon Event_Kind names (nemesis-core-ledger.ads)", () => {
    expect(eventLabel(0)).toBe("Mission Created");
    expect(eventLabel(3)).toBe("State Transitioned");
    expect(eventLabel(4)).toBe("Action Authorized");
    expect(eventLabel(6)).toBe("Evidence Accepted");
    expect(eventLabel(8)).toBe("Mission Completed");
    expect(eventLabel(9)).toBe("Mission Blocked");
  });

  it("falls back truthfully for codes outside the mirrored range", () => {
    expect(stateLabel(99)).toBe("STATE 99");
    expect(eventLabel(42)).toBe("KIND 42");
  });
});
