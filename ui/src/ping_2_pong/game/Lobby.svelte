<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getContext } from "svelte";
  import { createEventDispatcher } from "svelte";
  import type { AppClient, Record, HolochainError } from "@holochain/client";
  import { clientContext } from "../../contexts";
  import type { ClientContext } from "../../contexts";
  import CreateGame from "./CreateGame.svelte";
  import GlobalScoreboard from "./GlobalScoreboard.svelte";
  import type { Game } from "../ping_2_pong/types";

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  let availableGames: Record[] = [];
  let connectedPlayers: string[] = [];
  let loading: boolean = false;
  let error: string | null = null;
  
  // We'll hold the unsubscribe function here.
  let unsubscribe: () => void;

  // Fetch available games (filtering for games in Waiting state)
  async function fetchAvailableGames() {
    loading = true;
    error = null;
    try {
      const result = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_all_games",
        payload: null,
      });
      if (result) {
        availableGames = result.filter((r: Record) => {
          const entry = (r.entry as any)?.Present?.entry as Game;
          return entry && entry.game_status.type === "Waiting";
        });
      }
    } catch (e) {
      error = (e as HolochainError).message;
    } finally {
      loading = false;
    }
  }

  // Fetch connected players (simulate if no actual zome function)
  async function fetchConnectedPlayers() {
    try {
      // If your zome has a get_all_players, replace this simulation.
      connectedPlayers = ["Alice", "Bob"];
    } catch (e) {
      console.error("Error fetching players", e);
    }
  }

  // Subscribe to signals for live updates. This function is called only after client is defined.
  function subscribeToSignals() {
    return client.on("signal", (signal: any) => {
      console.log("Received signal:", signal);
      if (signal.type === "GameUpdate" || signal.type === "EntryCreated") {
        fetchAvailableGames();
      }
      // Optionally add player update signals.
    });
  }

  onMount(async () => {
    client = await appClientContext.getClient();
    await fetchAvailableGames();
    await fetchConnectedPlayers();
    // Now that client is defined, subscribe to signals.
    unsubscribe = subscribeToSignals();
  });

  onDestroy(() => {
    if (unsubscribe) unsubscribe();
  });

  function joinGame(gameRecord: Record) {
    dispatch("join-game", { gameHash: gameRecord.signed_action.hashed.hash });
  }

  // Helper to extract game ID string from a game record.
  function getGameIdString(gameRecord: Record): string {
    const gameEntry = (gameRecord.entry as any)?.Present?.entry as Game;
    return gameEntry ? gameEntry.game_id.toString() : "Unknown";
  }
</script>

<div class="lobby">
  <h2>Game Lobby</h2>
  <section class="game-list">
    {#if loading}
      <p>Loading available games…</p>
    {:else if error}
      <p class="error">Error: {error}</p>
    {:else if availableGames.length === 0}
      <p>No available games found. Create a new game!</p>
    {:else}
      <ul>
        {#each availableGames as gameRecord (gameRecord.signed_action.hashed.hash)}
          <li>
            <span>Game ID: {getGameIdString(gameRecord)}</span>
            <button on:click={() => joinGame(gameRecord)}>Join Game</button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="players-list">
    <h3>Connected Players</h3>
    {#if connectedPlayers.length === 0}
      <p>No players connected yet.</p>
    {:else}
      <ul>
        {#each connectedPlayers as player}
          <li>{player}</li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="create-game">
    <h3>Create a New Game</h3>
    <CreateGame on:game-created={() => fetchAvailableGames()} />
  </section>

  <section class="global-scoreboard">
    <GlobalScoreboard />
  </section>
</div>

<style>
  .lobby {
    padding: 1rem;
    max-width: 800px;
    margin: 0 auto;
    color: #fff;
  }
  .game-list, .players-list, .create-game, .global-scoreboard {
    margin-bottom: 2rem;
    background-color: #333;
    padding: 1rem;
    border-radius: 8px;
  }
  ul {
    list-style: none;
    padding: 0;
  }
  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
    padding: 0.5rem;
    border: 1px solid #555;
    border-radius: 4px;
  }
  button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    background-color: #646cff;
    color: #fff;
    cursor: pointer;
    transition: background-color 0.25s;
  }
  button:hover {
    background-color: #535bf2;
  }
  .error {
    color: #ff8080;
    text-align: center;
  }
</style>
