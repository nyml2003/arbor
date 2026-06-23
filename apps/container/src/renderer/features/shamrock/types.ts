export type ElementType =
  | "Grass"
  | "Fire"
  | "Water"
  | "Electric"
  | "Normal"
  | "Poison";

export type BattleSideView = Readonly<{
  label: "Player" | "Opponent";
  name: string;
  species: string;
  level: number;
  primaryType: ElementType;
  secondaryType: ElementType | null;
  hp: number;
  maxHp: number;
  status: "OK" | "PAR" | "PSN" | "BRN";
  partyAlive: number;
  partySize: number;
}>;

export type BattleActionView = Readonly<{
  id: string;
  label: string;
  element: ElementType;
  power: number | null;
  pp: string;
}>;

export type BattleLogLine = Readonly<{
  id: string;
  text: string;
}>;

export type ShamrockBattleView = Readonly<{
  battleId: string;
  turn: number;
  weather: string;
  player: BattleSideView;
  opponent: BattleSideView;
  actions: BattleActionView[];
  log: BattleLogLine[];
}>;

export type ShamrockSceneState = Readonly<{
  turn: number;
  playerHpRatio: number;
  opponentHpRatio: number;
  selectedActionIndex: number;
}>;
