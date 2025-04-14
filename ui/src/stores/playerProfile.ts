// ping2pong/ui/src/stores/playerProfile.ts
import { writable } from "svelte/store";
import type { AgentPubKey } from "@holochain/client"; // Import AgentPubKey type

export interface PlayerProfile {
  nickname: string;
  // FIX: Store the actual AgentPubKey object/Uint8Array
  agentKey: AgentPubKey;
}

// Initialize with null or load from storage if implementing persistence
export const playerProfile = writable<PlayerProfile | null>(null);