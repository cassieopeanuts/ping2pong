<script lang="ts">
import type { ActionHash, AgentPubKey, AppClient, DnaHash, EntryHash, HolochainError, Record } from "@holochain/client";
import { decode } from "@msgpack/msgpack";
import { createEventDispatcher, getContext, onMount } from "svelte";
import { type ClientContext, clientContext } from "../../contexts";
import EditGame from "./EditGame.svelte";
import type { Game, GameStatus } from "./types";

let client: AppClient;
const appClientContext = getContext<ClientContext>(clientContext);
const dispatch = createEventDispatcher();

let loading: boolean = false;
let editing = false;
let error: HolochainError | undefined;
let record: Record | undefined;
let game: Game | undefined;

export let gameHash: ActionHash;

$: editing, error, loading, record, game;

onMount(async () => {
  if (gameHash === undefined) {
    throw new Error(`The gameHash input is required for the GameDetail element`);
  }
  client = await appClientContext.getClient();
  await fetchGame();
});

async function fetchGame() {
  loading = true;
  try {
    record = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "get_latest_game",
      payload: gameHash,
    });
    if (record) {
      game = decode((record.entry as any).Present.entry) as Game;
    }
  } catch (e) {
    error = e as HolochainError;
  } finally {
    loading = false;
  }
}

async function deleteGame() {
  try {
    await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "delete_game",
      payload: gameHash,
    });
    dispatch("game-deleted", { gameHash: gameHash });
  } catch (e) {
    alert((e as HolochainError).message);
  }
}
</script>

{#if loading}
  <progress />
{:else if error}
  <div class="alert">Error fetching the game: {error.message}</div>
{:else if editing}
  <EditGame
    originalGameHash={gameHash}
    currentRecord={record}
    on:game-updated={async () => {
      editing = false;
      await fetchGame();
    }}
    on:edit-canceled={() => {
      editing = false;
    }}
  />
{:else}
  <section>
    <div>
      <span><strong>Game Status:</strong></span>
      <span>{
        game?.game_status.type === "Waiting"
        ? `Waiting`
        : game?.game_status.type === "InProgress"
        ? `In Progress`
        : `Finished`
      }</span>
    </div>

    <div>
      <button
        on:click={() => {
          editing = true;
        }}
      >edit</button>
      <button on:click={() => deleteGame()}>delete</button>
    </div>
  </section>
{/if}
