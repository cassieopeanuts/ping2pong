<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { decode } from "@msgpack/msgpack";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import type { Game, GameStatus } from "./types";

let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);
const dispatch = createEventDispatcher();

export let currentRecord!: Record;
export let originalGameHash!: ActionHash;

let currentGame: Game = decode((currentRecord.entry as any).Present.entry) as Game;
let gameStatus: GameStatus | undefined = currentGame.game_status;

$: gameStatus;
$: isGameValid = true && true;

onMount(async () => {
  if (!currentRecord) {
    throw new Error(`The currentRecord input is required for the EditGame element`);
  }
  if (!originalGameHash) {
    throw new Error(`The originalGameHash input is required for the EditGame element`);
  }
  client = await appClientContext.getClient();
});

async function updateGame() {
  const game: Game = {
    game_status: gameStatus!,
    game_id: currentGame.game_id,
    player_1: currentGame.player_1,
    player_2: currentGame.player_2,
    created_at: currentGame.created_at,
  };

  try {
    const updateRecord: Record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "update_game",
      payload: {
        original_game_hash: originalGameHash,
        previous_game_hash: currentRecord.signed_action.hashed.hash,
        updated_game: game,
      },
    });

    dispatch("game-updated", { actionHash: updateRecord.signed_action.hashed.hash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

<section>
  <div>
    <label for="Game Status">Game Status:</label>
    <select name="Game Status" bind:value={gameStatus?.type}>
      <option value="Waiting">Waiting</option>
      <option value="InProgress">In Progress</option>
      <option value="Finished">Finished</option>
    </select>
  </div>

  <div>
    <button on:click={() => dispatch("edit-canceled")}>Cancel</button>
    <button disabled={!isGameValid} on:click={() => updateGame()}>
      Edit Game
    </button>
  </div>
</section>
