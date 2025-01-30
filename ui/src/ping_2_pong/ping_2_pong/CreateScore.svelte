<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import type { Score } from "./types";

const dispatch = createEventDispatcher();
let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);

export let gameId!: string;
export let player!: AgentPubKey;
export let playerPoints!: number;

$: gameId, player, playerPoints;
$: isScoreValid = true;

onMount(async () => {
  if (gameId === undefined) {
    throw new Error(`The gameId input is required for the CreateScore element`);
  }
  if (player === undefined) {
    throw new Error(`The player input is required for the CreateScore element`);
  }
  if (playerPoints === undefined) {
    throw new Error(`The playerPoints input is required for the CreateScore element`);
  }
  client = await appClientContext.getClient();
});

async function createScore() {
  const scoreEntry: Score = {
    game_id: gameId!,
    player: player!,
    player_points: playerPoints!,
  };

  try {
    const record: Record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "create_score",
      payload: scoreEntry,
    });
    dispatch("score-created", { scoreHash: record.signed_action.hashed.hash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

<div>
  <h3>Create Score</h3>

  <button disabled={!isScoreValid} on:click={() => createScore()}>
    Create Score
  </button>
</div>
