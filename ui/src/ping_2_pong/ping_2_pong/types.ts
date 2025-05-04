// ping_2_pong/ui/src/ping_2_pong/types.ts

import type {
  ActionHash,
  AgentPubKey,
  Create,
  CreateLink,
  Delete,
  DeleteLink,
  SignedActionHashed,
  Update,
  Entry,
  Record, // Added Record type
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

// String literal unions matching backend enums
export type PlayerStatus = 'Available' | 'InGame';
export type GameStatus = 'Waiting' | 'InProgress' | 'Finished';

// --- SIGNAL TYPES ---

// Standard Holochain signals (emitted by post_commit hook)
export type EntryCreatedSignal = {
  type: "EntryCreated";
  action: SignedActionHashed<Create>;
  app_entry: EntryTypes;
};
export type EntryUpdatedSignal = {
  type: "EntryUpdated";
  action: SignedActionHashed<Update>;
  app_entry: EntryTypes;
  original_app_entry: EntryTypes;
};
export type EntryDeletedSignal = {
  type: "EntryDeleted";
  action: SignedActionHashed<Delete>;
  original_app_entry: EntryTypes;
};
export type LinkCreatedSignal = {
  type: "LinkCreated";
  action: SignedActionHashed<CreateLink>;
  link_type: LinkTypes;
};
export type LinkDeletedSignal = {
  type: "LinkDeleted";
  action: SignedActionHashed<DeleteLink>;
  link_type: LinkTypes;
  create_link_action: SignedActionHashed<CreateLink>;
};

// Custom application signals
export type GameInvitationSignal = {
  type: "GameInvitation";
  game_id: ActionHash;
  inviter: AgentPubKey;
  message: string;
};
export type PaddleUpdateSignal = {
   type: "PaddleUpdate";
   game_id: ActionHash;
   player: AgentPubKey;
   paddle_y: number; // Corresponds to u32
};
export type BallUpdateSignal = {
   type: "BallUpdate";
   game_id: ActionHash;
   ball_x: number; // Corresponds to u32
   ball_y: number; // Corresponds to u32
   ball_dx: number; // Corresponds to i32
   ball_dy: number; // Corresponds to i32
};
export type GameOverSignal = {
    type: "GameOver";
    game_id: ActionHash;
    winner: AgentPubKey | null; // Corresponds to Option<AgentPubKey>
    score1: number; // Corresponds to u32
    score2: number; // Corresponds to u32
};

// *** ADDED GameStartedSignal definition ***
export type GameStartedSignal = {
  type: "GameStarted";
  game_id: ActionHash;
  player_1: AgentPubKey; // Player 1's public key
  player_2: AgentPubKey; // Player 2's public key
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
 | GameOverSignal
 | GameStartedSignal; // Added GameStartedSignal to the union


// --- ENTRY TYPES ---

/* dprint-ignore-start */
// Define EntryTypes matching the integrity zome enum variants
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
  player_2: AgentPubKey | null; // Corresponds to Option<AgentPubKey>
  created_at: number; // Holochain Timestamp might need conversion if not just number
  game_status: GameStatus; // Use string union type
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
  created_at: number; // Holochain Timestamp -> number (check backend serialization)
}

export interface Statistics {
  game_id: ActionHash;
  signal_latency: number; // u32 -> number
  score_validation_time: number; // u32 -> number
  dht_response_time: number; // u32 -> number
  network_delay: number; // u32 -> number
  timestamp: number; // Holochain Timestamp -> number (check backend serialization)
}

export interface Presence {
    agent_pubkey: AgentPubKey;
    timestamp: number; // Corresponds to u64
}

export interface AnchorPath {
    // No specific fields needed by UI based on current usage
}

// Input type for update_game zome call
export interface UpdateGameInput {
  original_game_hash: ActionHash;
  previous_game_hash: ActionHash;
  updated_game: Game;
}

// Input type for send_invitation zome call (matches Rust struct)
export interface Invitation {
    game_id: ActionHash;
    inviter: AgentPubKey;
    message: string;
}

