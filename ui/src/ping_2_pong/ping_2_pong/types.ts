// ping_2_pong/ui/src/ping_2_pong/types.ts

import type {
  ActionHash,
  AgentPubKey,
  Create,
  CreateLink,
  Delete,
  DeleteLink,
  // DnaHash, // Currently unused
  // EntryHash, // Currently unused directly by UI types
  // ExternalHash, // Currently unused
  // Record, // Used in components, but not directly in these shared types
  SignedActionHashed,
  Update,
  Entry, // Import Entry type for Ping2PongSignal if needed, or use unknown
} from "@holochain/client";

// --- ENUMS ---

// Mirror Rust enum from integrity/lib.rs
export enum LinkTypes {
    GameIdToGame = 'GameIdToGame',
    Player1ToGames = 'Player1ToGames',
    Player2ToGames = 'Player2ToGames',
    GameUpdates = 'GameUpdates',
    GameToScores = 'GameToScores',
    GameToStatistics = 'GameToStatistics',
    PlayerToPlayers = 'PlayerToPlayers',
    PlayerNameToPlayer = 'PlayerNameToPlayer',
    PlayerUpdates = 'PlayerUpdates',
    PlayerToScores = 'PlayerToScores',
    Presence = 'Presence',
}

// As defined in coordinator/game.rs
export type PlayerStatus =
  | { type: 'Available'; }
  | { type: 'InGame'; };

// As defined in integrity/game.rs
export type GameStatus =
  | { type: 'Waiting'; }
  | { type: 'InProgress'; }
  | { type: 'Finished'; };


// --- SIGNAL TYPES ---

// Standard Holochain signals (emitted by post_commit hook)
// Using 'unknown' for app_entry might be simpler if exact structure isn't always needed for signals
export type EntryCreatedSignal = {
  type: "EntryCreated";
  action: SignedActionHashed<Create>;
  app_entry: EntryTypes; // Or 'unknown'/'any' if type detail isn't needed by signal consumer
};
export type EntryUpdatedSignal = {
  type: "EntryUpdated";
  action: SignedActionHashed<Update>;
  app_entry: EntryTypes; // Or 'unknown'/'any'
  original_app_entry: EntryTypes; // Or 'unknown'/'any'
};
export type EntryDeletedSignal = {
  type: "EntryDeleted";
  action: SignedActionHashed<Delete>;
  original_app_entry: EntryTypes; // Or 'unknown'/'any'
};
export type LinkCreatedSignal = {
  type: "LinkCreated";
  action: SignedActionHashed<CreateLink>;
  link_type: LinkTypes; // Use defined enum
};
export type LinkDeletedSignal = {
  type: "LinkDeleted";
  action: SignedActionHashed<DeleteLink>;
  link_type: LinkTypes; // Use defined enum
  // Note: create_link_action was present in backend Signal enum, add if needed
  create_link_action: SignedActionHashed<CreateLink>;
};

// Custom application signals (sent via send_signal / received via client.on('signal'))
export type GameInvitationSignal = {
  type: "GameInvitation"; // Matches payload type field
  game_id: ActionHash;
  inviter: AgentPubKey;
  message: string;
};
export type PaddleUpdateSignal = {
   type: "PaddleUpdate"; // Matches payload type field
   game_id: ActionHash;
   player: AgentPubKey;
   paddle_y: number;
};
export type BallUpdateSignal = {
   type: "BallUpdate"; // Matches payload type field
   game_id: ActionHash;
   ball_x: number;
   ball_y: number;
   ball_dx: number;
   ball_dy: number;
};
export type GameOverSignal = {
    type: "GameOver"; // Matches payload type field
    game_id: ActionHash;
    winner: AgentPubKey | null; // Winner pubkey or null
    score1: number;
    score2: number;
};

// Union of all possible signals the UI might receive/handle
export type Ping2PongSignal =
 | EntryCreatedSignal
 | EntryUpdatedSignal
 | EntryDeletedSignal
 | LinkCreatedSignal
 | LinkDeletedSignal
 | GameInvitationSignal
 | PaddleUpdateSignal
 | BallUpdateSignal
 | GameOverSignal;


// --- ENTRY TYPES ---

/* dprint-ignore-start */
// Define EntryTypes matching the integrity zome enum variants
// The '& { field: Type }' pattern represents the Rust struct content.
export type EntryTypes =
 | ({ type: 'Game'; } & Game)
 | ({ type: 'Player'; } & Player)
 | ({ type: 'Score'; } & Score)
 | ({ type: 'Statistics'; } & Statistics)
 | ({ type: 'Presence'; } & Presence)
 | ({ type: 'AnchorPath'; } & AnchorPath);
/* dprint-ignore-end */


// --- Data Structure Interfaces (matching Rust structs) ---

export interface Game {
  player_1: AgentPubKey;
  player_2: AgentPubKey | null;
  created_at: number; // Holochain Timestamp (usually [secs, nanos]) might need conversion if not just number
  game_status: GameStatus;
  player_1_paddle: number; // u32 -> number
  player_2_paddle: number; // u32 -> number
  ball_x: number; // u32 -> number
  ball_y: number; // u32 -> number
}

export interface Player {
  player_key: AgentPubKey;
  player_name: string;
}

export interface Score {
  game_id: ActionHash;
  player: AgentPubKey;
  player_points: number; // u32 -> number
  created_at: number; // Holochain Timestamp -> number
}

export interface Statistics {
  game_id: ActionHash;
  signal_latency: number; // u32 -> number
  score_validation_time: number; // u32 -> number (consider renaming based on meaning)
  dht_response_time: number; // u32 -> number
  network_delay: number; // u32 -> number
  timestamp: number; // Holochain Timestamp -> number
}

export interface Presence {
    agent_pubkey: AgentPubKey;
    timestamp: number; // Corresponds to i64 -> number
}

export interface AnchorPath {
    // Empty is fine if content isn't needed client-side
    // path_string?: string; // Optionally add if Path serializes to string
}

// Input type for zome calls
export interface UpdateGameInput {
  original_game_hash: ActionHash;
  previous_game_hash: ActionHash;
  updated_game: Game;
}

// Note: Invitation struct from backend is covered by GameInvitationSignal type