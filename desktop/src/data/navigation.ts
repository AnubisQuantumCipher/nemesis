export const navigation = [
  ["Home", "HM"],
  ["Missions", "MS"],
  ["Evidence", "EV"],
  ["Replay", "RP"],
  ["Settings", "ST"],
] as const;

export type NavigationName = (typeof navigation)[number][0];
