export const navigation = [
  ["Home", "HM"],
  ["Missions", "MS"],
  ["Workspaces", "WS"],
  ["Agents", "AG"],
  ["Changes", "CH"],
  ["Tests", "TS"],
  ["Evidence", "EV"],
  ["Knowledge", "KN"],
  ["Skills", "SK"],
  ["Automations", "AU"],
  ["Integrations", "IN"],
  ["Security", "SC"],
  ["Replay", "RP"],
  ["Settings", "ST"],
] as const;

export type NavigationName = (typeof navigation)[number][0];
