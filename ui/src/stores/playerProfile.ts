import { writable } from "svelte/store";

export interface PlayerProfile {
  nickname: string;
  // You might also include the agent public key if needed:
  agentKey: string;
}

export const playerProfile = writable<PlayerProfile | null>(null);
