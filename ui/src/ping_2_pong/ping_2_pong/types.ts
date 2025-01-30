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
};

/* dprint-ignore-start */
export type EntryTypes =
 | ({ type: 'Statistics'; } & Statistics)
 | ({ type: 'Score'; } & Score)
 | ({ type: 'Player'; } & Player)
 | ({  type: 'Game'; } & Game);
/* dprint-ignore-end */

export type GameStatus = {type: 'Waiting'} | {type: 'InProgress'} | {type: 'Finished'};

export interface Game {
  game_id: string;
  player_1: AgentPubKey;
  player_2: AgentPubKey;
  created_at: number;
  game_status: GameStatus;
}

export interface Player {
  player_key: AgentPubKey;
  player_name: string;
}

export interface Score {
  game_id: string;
  player: AgentPubKey;
  player_points: number;
}

export interface Statistics {
  game_id: string;
  signal_latency: number;
  score_validation_time: number;
  dht_response_time: number;
  network_delay: number;
  timestamp: number;
}
