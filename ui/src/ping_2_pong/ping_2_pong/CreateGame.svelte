<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import type { Game, GameStatus } from "./types";

const dispatch = createEventDispatcher();
let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);

let gameStatus: GameStatus = { type: "Waiting" };

export let gameId!: string;
export let player1!: AgentPubKey;
export let player2!: AgentPubKey;
export let createdAt!: number;

$: gameId, player1, player2, createdAt, gameStatus;
$: isGameValid = true && true;

onMount(async () => {
  if (gameId === undefined) {
    throw new Error(`The gameId input is required for the CreateGame element`);
  }
  if (player1 === undefined) {
    throw new Error(`The player1 input is required for the CreateGame element`);
  }
  if (player2 === undefined) {
    throw new Error(`The player2 input is required for the CreateGame element`);
  }
  if (createdAt === undefined) {
    throw new Error(`The createdAt input is required for the CreateGame element`);
  }
  client = await appClientContext.getClient();
});

async function createGame() {
  const gameEntry: Game = {
    game_id: gameId!,
    player_1: player1!,
    player_2: player2!,
    created_at: createdAt!,
    game_status: gameStatus!,
  };

  try {
    const record: Record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "create_game",
      payload: gameEntry,
    });
    dispatch("game-created", { gameHash: record.signed_action.hashed.hash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

<div>
  <h3>Create Game</h3>

  <div>
    <label for="Game Status">Game Status:</label>
    <select name="Game Status" bind:value={gameStatus?.type}>
      <option value="Waiting">Waiting</option>
      <option value="InProgress">In Progress</option>
      <option value="Finished">Finished</option>
    </select>
  </div>

  <button disabled={!isGameValid} on:click={() => createGame()}>
    Create Game
  </button>
</div>
