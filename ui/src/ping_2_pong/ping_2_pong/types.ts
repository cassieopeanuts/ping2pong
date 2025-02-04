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

// Note: now game_id is an ActionHash (as per DNA)
export type Ping2PongSignal = {
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
  link_type: string;
} | {
  type: "LinkDeleted";
  action: SignedActionHashed<DeleteLink>;
  link_type: string;
} | {
  // New variant for real-time game updates.
  type: "GameUpdate";
  game_id: ActionHash;
  paddle1: number;
  paddle2: number;
  ball_x: number;
  ball_y: number;
};

/* dprint-ignore-start */
export type EntryTypes =
 | ({ type: 'Statistics'; } & Statistics)
 | ({ type: 'Score'; } & Score)
 | ({ type: 'Player'; } & Player)
 | ({ type: 'Game'; } & Game);
/* dprint-ignore-end */

export type GameStatus = {type: 'Waiting'} | {type: 'InProgress'} | {type: 'Finished'};

export interface Game {
  // Now game_id is an ActionHash (but you might store it as string if you convert it)
  game_id: ActionHash;
  player_1: AgentPubKey;
  player_2: AgentPubKey;
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
}

export interface Statistics {
  game_id: ActionHash;
  signal_latency: number;
  score_validation_time: number;
  dht_response_time: number;
  network_delay: number;
  timestamp: number;
}

export interface LobbyData {
  games: Game[];
}
