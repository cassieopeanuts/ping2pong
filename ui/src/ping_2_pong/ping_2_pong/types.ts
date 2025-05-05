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
  Record,
} from "@holochain/client";

/* ─────────────── ENUMS ─────────────── */

export enum LinkTypes {
  GameIdToGame      = "GameIdToGame",
  Player1ToGames    = "Player1ToGames",
  Player2ToGames    = "Player2ToGames",
  GameUpdates       = "GameUpdates",
  GameToScores      = "GameToScores",
  GameToStatistics  = "GameToStatistics",
  PlayerToPlayers   = "PlayerToPlayers",
  PlayerNameToPlayer= "PlayerNameToPlayer",
  PlayerUpdates     = "PlayerUpdates",
  PlayerToScores    = "PlayerToScores",
  Presence          = "Presence",
}

export type PlayerStatus = "Available" | "InGame";
export type GameStatus   = "Waiting"   | "InProgress" | "Finished";

/* ─────────────── Holochain-generated signals ─────────────── */

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

/* ─────────────── Custom app signals ─────────────── */

export type GameInvitationSignal = {
  type: "GameInvitation";
  game_id: ActionHash;
  inviter: AgentPubKey;
  message: string;
};

export type PaddleUpdateSignal = {
  type: "PaddleUpdate";
  game_id: ActionHash;
  player:   AgentPubKey;
  paddle_y: number;
};

export type BallUpdateSignal = {
  type: "BallUpdate";
  game_id: ActionHash;
  ball_x:  number;
  ball_y:  number;
  ball_dx: number;
  ball_dy: number;
};

export type ScoreUpdateSignal = {            // ← NEW
  type: "ScoreUpdate";
  game_id: ActionHash;
  score1 : number;
  score2 : number;
};

export type GameOverSignal = {
  type: "GameOver";
  game_id: ActionHash;
  winner: AgentPubKey | null;
  score1: number;
  score2: number;
};

export type GameStartedSignal = {
  type: "GameStarted";
  game_id: ActionHash;
  player_1: AgentPubKey;
  player_2: AgentPubKey;
};

/* ─────────────── Union of every signal the UI handles ─────────────── */

export type Ping2PongSignal =
  | EntryCreatedSignal
  | EntryUpdatedSignal
  | EntryDeletedSignal
  | LinkCreatedSignal
  | LinkDeletedSignal
  | GameInvitationSignal
  | PaddleUpdateSignal
  | BallUpdateSignal
  | ScoreUpdateSignal         // ← NEW
  | GameOverSignal
  | GameStartedSignal;

/* ─────────────── Entry-type mappings ─────────────── */
/* dprint-ignore-start */
export type EntryTypes =
  | ({ type: "Game"; }        & Game)
  | ({ type: "Player"; }      & Player)
  | ({ type: "Score"; }       & Score)
  | ({ type: "Statistics"; }  & Statistics)
  | ({ type: "Presence"; }    & Presence)
  | ({ type: "AnchorPath"; }  & AnchorPath);
/* dprint-ignore-end */

/* ─────────────── Data structures ─────────────── */

export interface Game {
  player_1: AgentPubKey;
  player_2: AgentPubKey | null;
  created_at: number;
  game_status: GameStatus;
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
  game_id: ActionHash;
  player: AgentPubKey;
  player_points: number;
  created_at: number;
}

export interface Statistics {
  game_id: ActionHash;
  signal_latency: number;
  score_validation_time: number;
  dht_response_time: number;
  network_delay: number;
  timestamp: number;
}

export interface Presence {
  agent_pubkey: AgentPubKey;
  timestamp: number;
}

export interface AnchorPath {}

/* ─────────────── Helper payloads ─────────────── */

export interface UpdateGameInput {
  original_game_hash: ActionHash;
  previous_game_hash: ActionHash;
  updated_game: Game;
}

export interface Invitation {
  game_id: ActionHash;
  inviter: AgentPubKey;
  message: string;
}
