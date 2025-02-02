// This element was auto created by Holochain scaffolding tool and we dont really need it
// We keep it for archive purpose

<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { decode } from "@msgpack/msgpack";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import type { Score } from "./types";

let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);
const dispatch = createEventDispatcher();

export let currentRecord!: Record;
export let originalScoreHash!: ActionHash;

let currentScore: Score = decode((currentRecord.entry as any).Present.entry) as Score;

$: ;
$: isScoreValid = true;

onMount(async () => {
  if (!currentRecord) {
    throw new Error(`The currentRecord input is required for the EditScore element`);
  }
  if (!originalScoreHash) {
    throw new Error(`The originalScoreHash input is required for the EditScore element`);
  }
  client = await appClientContext.getClient();
});

async function updateScore() {
  const score: Score = {
    game_id: currentScore.game_id,
    player: currentScore.player,
    player_points: currentScore.player_points,
  };

  try {
    const updateRecord: Record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "update_score",
      payload: {
        original_score_hash: originalScoreHash,
        previous_score_hash: currentRecord.signed_action.hashed.hash,
        updated_score: score,
      },
    });

    dispatch("score-updated", { actionHash: updateRecord.signed_action.hashed.hash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

<section>
  <div>
    <button on:click={() => dispatch("edit-canceled")}>Cancel</button>
    <button disabled={!isScoreValid} on:click={() => updateScore()}>
      Edit Score
    </button>
  </div>
</section>
