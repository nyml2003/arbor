import type { Clock } from "../domain/types.js";

export const systemClock: Clock = () => new Date().toISOString();
