import {
  ActionHash,
  AppBundleSource,
  fakeActionHash,
  fakeAgentPubKey,
  fakeDnaHash,
  fakeEntryHash,
  hashFrom32AndType,
  NewEntryAction,
  Record,
} from "@holochain/client";
import { CallableCell } from "@holochain/tryorama";

export async function sampleGame(cell: CallableCell, partialGame = {}) {
  return {
    ...{
      game_id: "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
      player_1: cell.cell_id[1],
      player_2: cell.cell_id[1],
      created_at: 1674053334548000,
      game_status: { type: "Waiting" },
    },
    ...partialGame,
  };
}

export async function createGame(cell: CallableCell, game = undefined): Promise<Record> {
  return cell.callZome({
    zome_name: "ping_2_pong",
    fn_name: "create_game",
    payload: game || await sampleGame(cell),
  });
}

export async function samplePlayer(cell: CallableCell, partialPlayer = {}) {
  return {
    ...{
      player_key: cell.cell_id[1],
      player_name: "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
    },
    ...partialPlayer,
  };
}

export async function createPlayer(cell: CallableCell, player = undefined): Promise<Record> {
  return cell.callZome({
    zome_name: "ping_2_pong",
    fn_name: "create_player",
    payload: player || await samplePlayer(cell),
  });
}

export async function sampleScore(cell: CallableCell, partialScore = {}) {
  return {
    ...{
      game_id: "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
      player: cell.cell_id[1],
      player_points: 10,
    },
    ...partialScore,
  };
}

export async function createScore(cell: CallableCell, score = undefined): Promise<Record> {
  return cell.callZome({
    zome_name: "ping_2_pong",
    fn_name: "create_score",
    payload: score || await sampleScore(cell),
  });
}

export async function sampleStatistics(cell: CallableCell, partialStatistics = {}) {
  return {
    ...{
      game_id: "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
      signal_latency: 10,
      score_validation_time: 10,
      dht_response_time: 10,
      network_delay: 10,
      timestamp: 1674053334548000,
    },
    ...partialStatistics,
  };
}

export async function createStatistics(cell: CallableCell, statistics = undefined): Promise<Record> {
  return cell.callZome({
    zome_name: "ping_2_pong",
    fn_name: "create_statistics",
    payload: statistics || await sampleStatistics(cell),
  });
}
