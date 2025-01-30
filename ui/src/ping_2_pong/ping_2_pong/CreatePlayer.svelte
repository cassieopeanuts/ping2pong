<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import type { Player } from "./types";

const dispatch = createEventDispatcher();
let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);

export let playerKey!: AgentPubKey;
export let playerName!: string;

$: playerKey, playerName;
$: isPlayerValid = true;

onMount(async () => {
  if (playerKey === undefined) {
    throw new Error(`The playerKey input is required for the CreatePlayer element`);
  }
  if (playerName === undefined) {
    throw new Error(`The playerName input is required for the CreatePlayer element`);
  }
  client = await appClientContext.getClient();
});

async function createPlayer() {
  const playerEntry: Player = {
    player_key: playerKey!,
    player_name: playerName!,
  };

  try {
    const record: Record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "create_player",
      payload: playerEntry,
    });
    dispatch("player-created", { playerHash: record.signed_action.hashed.hash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

<div>
  <h3>Create Player</h3>

  <button disabled={!isPlayerValid} on:click={() => createPlayer()}>
    Create Player
  </button>
</div>
