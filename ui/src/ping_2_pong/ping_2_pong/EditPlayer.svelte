<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { decode } from "@msgpack/msgpack";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import type { Player } from "./types";

let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);
const dispatch = createEventDispatcher();

export let currentRecord!: Record;
export let originalPlayerHash!: ActionHash;

let currentPlayer: Player = decode((currentRecord.entry as any).Present.entry) as Player;

$: ;
$: isPlayerValid = true;

onMount(async () => {
  if (!currentRecord) {
    throw new Error(`The currentRecord input is required for the EditPlayer element`);
  }
  if (!originalPlayerHash) {
    throw new Error(`The originalPlayerHash input is required for the EditPlayer element`);
  }
  client = await appClientContext.getClient();
});

async function updatePlayer() {
  const player: Player = {
    player_key: currentPlayer.player_key,
    player_name: currentPlayer.player_name,
  };

  try {
    const updateRecord: Record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "update_player",
      payload: {
        original_player_hash: originalPlayerHash,
        previous_player_hash: currentRecord.signed_action.hashed.hash,
        updated_player: player,
      },
    });

    dispatch("player-updated", { actionHash: updateRecord.signed_action.hashed.hash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

<section>
  <div>
    <button on:click={() => dispatch("edit-canceled")}>Cancel</button>
    <button disabled={!isPlayerValid} on:click={() => updatePlayer()}>
      Edit Player
    </button>
  </div>
</section>
