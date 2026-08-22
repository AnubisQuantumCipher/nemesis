export const navigation = [
  ["Home", "HM"],
  ["Missions", "MS"],
  ["Workspaces", "WS"],
  ["Agents", "AG"],
  ["Changes", "CH"],
  ["Tests", "TS"],
  ["Knowledge", "KN"],
  ["Skills", "SK"],
  ["Automations", "AU"],
  ["Integrations", "IN"],
  ["Security", "SC"],
  ["Evidence", "EV"],
  ["Replay", "RP"],
  ["Settings", "ST"],
] as const;

export type NavigationName = (typeof navigation)[number][0];

/** Rails backed by the governed state repository and derived views. */
export const railNames = [
  "Workspaces",
  "Agents",
  "Changes",
  "Tests",
  "Knowledge",
  "Skills",
  "Automations",
  "Integrations",
  "Security",
] as const;

export type RailName = (typeof railNames)[number];

export function isRailName(name: NavigationName): name is RailName {
  return (railNames as readonly string[]).includes(name);
}
