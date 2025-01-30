<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { decode } from "@msgpack/msgpack";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import EditPlayer from "./EditPlayer.svelte";
import type { Player } from "./types";

let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);
const dispatch = createEventDispatcher();

let loading: boolean = false;
let editing = false;
let error: HolochainError | undefined;
let record: Record | undefined;
let player: Player | undefined;

export let playerHash: ActionHash;

$: editing, error, loading, record, player;

onMount(async () => {
  if (playerHash === undefined) {
    throw new Error(`The playerHash input is required for the PlayerDetail element`);
  }
  client = await appClientContext.getClient();
  await fetchPlayer();
});

async function fetchPlayer() {
  loading = true;
  try {
    record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "get_latest_player",
      payload: playerHash,
    });
    if (record) {
      player = decode((record.entry as any).Present.entry) as Player;
    }
  } catch (e) {
    error = e as HolochainError;
  } finally {
    loading = false;
  }
}

async function deletePlayer() {
  try {
    await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "delete_player",
      payload: playerHash,
    });
    dispatch("player-deleted", { playerHash: playerHash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

{#if loading}
  <progress />
{:else if error}
  <div class="alert">Error fetching the player: {error.message}</div>
{:else if editing}
  <EditPlayer
    originalPlayerHash={playerHash}
    currentRecord={record}
    on:player-updated={async () => {
      editing = false;
      await fetchPlayer();
    }}
    on:edit-canceled={() => {
      editing = false;
    }}
  />
{:else}
  <section>
    <div>
      <button
        on:click={() => {
          editing = true;
        }}
      >edit</button>
      <button on:click={() => deletePlayer()}>delete</button>
    </div>
  </section>
{/if}
