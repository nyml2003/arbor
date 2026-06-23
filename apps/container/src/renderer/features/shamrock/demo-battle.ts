import type { ShamrockBattleView } from "./types";

export const demoBattle: ShamrockBattleView = {
  battleId: "first-playable-demo",
  turn: 18,
  weather: "Clear",
  player: {
    label: "Player",
    name: "Leafy",
    species: "Sproutaur",
    level: 50,
    primaryType: "Grass",
    secondaryType: "Poison",
    hp: 807,
    maxHp: 1145,
    status: "OK",
    partyAlive: 4,
    partySize: 4,
  },
  opponent: {
    label: "Opponent",
    name: "Blaze",
    species: "Flarekid",
    level: 50,
    primaryType: "Fire",
    secondaryType: null,
    hp: 776,
    maxHp: 1139,
    status: "OK",
    partyAlive: 3,
    partySize: 4,
  },
  actions: [
    { id: "vine-whip", label: "Vine Whip", element: "Grass", power: 45, pp: "22/25" },
    { id: "poison-powder", label: "Poison Powder", element: "Poison", power: null, pp: "33/35" },
    { id: "growl", label: "Growl", element: "Normal", power: null, pp: "40/40" },
    { id: "tackle", label: "Tackle", element: "Normal", power: 40, pp: "35/35" },
  ],
  log: [
    { id: "turn", text: "Resolving turn 18." },
    { id: "enemy", text: "Blaze used Ember." },
    { id: "player", text: "Leafy is waiting for a command." },
  ],
};

export function hpRatio(hp: number, maxHp: number): number {
  if (maxHp <= 0) return 0;
  return Math.max(0, Math.min(1, hp / maxHp));
}
