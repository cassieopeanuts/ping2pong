// Base64 encoded representation of an Agent's Public Key
export type AgentPubKeyB64 = string;

// Holochain's Timestamp is a struct (seconds: i64, nanoseconds: u32).
// For UI purposes, we'll represent it as a number (milliseconds since epoch).
// The conversion from the DNA's Timestamp struct to this number format
// will be handled when the signal is processed in the UI (e.g., in App.svelte).
export type HdkTimestamp = number;

export interface GlobalChatMessageSignal {
  type: "GlobalChatMessage";
  timestamp: HdkTimestamp; // Milliseconds since epoch
  sender: AgentPubKeyB64;
  content: string;
}

// You might also want to define the payload separately if it's used elsewhere
export interface ChatMessagePayloadU { // U for UI
  timestamp: HdkTimestamp;
  sender: AgentPubKeyB64;
  content: string;
}

import type { AgentPubKey } from '@holochain/client';

export interface Player {
  player_name: string;
  player_key: AgentPubKey; // Raw AgentPubKey (Uint8Array) as it's stored in the entry
}
