// ping_2_pong/ui/src/ping_2_pong/types.ts

import type {
  ActionHash,
  AgentPubKey,
  Create,
  CreateLink,
  Delete,
  DeleteLink,
  DnaHash,
  EntryHash,
  ExternalHash,
  Record,
  SignedActionHashed,
  Update,
} from "@holochain/client";

// --- ADD PlayerStatus TYPE DEFINITION ---
export type PlayerStatus =
  | { type: 'Available'; }
  | { type: 'InGame'; };
// -----------------------------------------


// Note: now game_id is an ActionHash (as per DNA)
// Consider if Ping2PongSignal needs update based on actual signals sent/received
// E.g., GameInvitation, PaddleUpdate, BallUpdate might come via client.on('signal')
export type Ping2PongSignal = /* ... existing signal types ... */
 {
  type: "EntryCreated";
  action: SignedActionHashed<Create>;
  app_entry: EntryTypes;
} | {
  type: "EntryUpdated";
  action: SignedActionHashed<Update>;
  app_entry: EntryTypes;
  original_app_entry: EntryTypes;
} | {
  type: "EntryDeleted";
  action: SignedActionHashed<Delete>;
  original_app_entry: EntryTypes;
} | {
  type: "LinkCreated";
  action: SignedActionHashed<CreateLink>;
  link_type: string; // Or define LinkTypes enum here too?
} | {
  type: "LinkDeleted";
  action: SignedActionHashed<DeleteLink>;
  link_type: string; // Or define LinkTypes enum here too?
} | {
  // Custom signal type for invitations (as sent from Lobby.svelte)
  type: "GameInvitation";
  game_id: ActionHash;
  inviter: AgentPubKey;
  message: string;
} | {
   // Custom signal type for paddle updates (as sent from PongGame.svelte)
   type: "PaddleUpdate";
   game_id: ActionHash;
   player: AgentPubKey;
   paddle_y: number;
 } | {
   // Custom signal type for ball updates (as sent from PongGame.svelte)
   type: "BallUpdate";
   game_id: ActionHash;
   ball_x: number;
   ball_y: number;
   ball_dx: number;
   ball_dy: number;
 }
;
 // REMOVED GameUpdate from this union as backend doesn't send it


/* dprint-ignore-start */
// Define EntryTypes matching the integrity zome enum variants
export type EntryTypes =
 | ({ type: 'Game'; } & Game) // Assumes Game struct is defined below
 | ({ type: 'Player'; } & Player) // Assumes Player struct is defined below
 | ({ type: 'Score'; } & Score) // Assumes Score struct is defined below
 | ({ type: 'Statistics'; } & Statistics) // Assumes Statistics struct is defined below
 | ({ type: 'Presence'; } & Presence) // Add Presence type
 | ({ type: 'AnchorPath'; } & AnchorPath); // Add AnchorPath type
/* dprint-ignore-end */


// --- Data Structure Interfaces/Types ---

export type GameStatus =
  | { type: 'Waiting'; }
  | { type: 'InProgress'; }
  | { type: 'Finished'; };

export interface Game {
  // game_id is created on the backend, not part of input usually
  // game_id: ActionHash; <-- Removed, ActionHash is the identifier, not usually part of the entry data itself
  player_1: AgentPubKey;
  player_2: AgentPubKey | null; // Allow null for waiting state
  created_at: number; // Corresponds to Timestamp
  game_status: GameStatus;
  // Default/Snapshot positions
  player_1_paddle: number;
  player_2_paddle: number;
  ball_x: number;
  ball_y: number;
}

export interface Player {
  player_key: AgentPubKey;
  player_name: string;
}

export interface Score {
  game_id: ActionHash; // Reference to the original game action hash
  player: AgentPubKey;
  player_points: number;
  created_at: number; // Added, likely present from backend validation
}

export interface Statistics {
  game_id: ActionHash; // Reference to the original game action hash
  signal_latency: number;
  score_validation_time: number; // Or renamed based on actual meaning
  dht_response_time: number;
  network_delay: number;
  timestamp: number; // Corresponds to Timestamp
}

// Add Presence type definition matching backend
export interface Presence {
    agent_pubkey: AgentPubKey;
    timestamp: number; // Corresponds to i64 timestamp (number in JS)
}

// Add AnchorPath type definition matching backend
export interface AnchorPath {
    // Path might serialize complexly, often represented just by its string form if needed client-side
    // Or simply don't need its fields if only used internally
    // Example: If Path serializes to its string representation:
    // path_string: string;
    // For now, leave empty if fields aren't needed in UI types
}


// Type for LobbyData seems unused, can remove if not needed
// export interface LobbyData {
//   games: Game[];
// }