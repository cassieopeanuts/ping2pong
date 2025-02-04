<script lang="ts">
    import { onMount, createEventDispatcher } from "svelte";
    import { getContext } from "svelte";
    import type { AppClient, Record, HolochainError } from "@holochain/client";
    import { clientContext } from "../../contexts";
    import type { ClientContext } from "../../contexts"; // Import our ClientContext type
    import CreateGame from "./CreateGame.svelte";
    import GlobalScoreboard from "./GlobalScoreboard.svelte";
    import type { Game } from "../ping_2_pong/types";
    
    const dispatch = createEventDispatcher();
    let client: AppClient;
    // Specify the context type so getClient is known to exist.
    const appClientContext = getContext<ClientContext>(clientContext);
    
    let availableGames: Record[] = [];
    let loading: boolean = false;
    let error: string | null = null;
    
    // Global anchor for games – this should be the same as in your DNA.
    const gamesAnchor = "games";
    
    // Helper function to extract game_id from a Record
    function getGameId(record: Record): string {
      const entry = (record.entry as any)?.Present?.entry as Game;
      return entry ? entry.game_id.toString() : "Unknown";
    }
      
    async function fetchAvailableGames() {
      loading = true;
      error = null;
      try {
        const result = await client.callZome({
          cap_secret: null,
          role_name: "ping_2_pong",
          zome_name: "ping_2_pong",
          fn_name: "get_all_games",
          payload: gamesAnchor,
        });
        if (result) {
          availableGames = result.filter((r) => {
            const game = (r.entry as any)?.Present?.entry;
            return game && game.game_status.type === "Waiting";
          });
        }
      } catch (e) {
        error = (e as HolochainError).message;
      } finally {
        loading = false;
      }
    }
    
    onMount(async () => {
      client = await appClientContext.getClient();
      await fetchAvailableGames();
    });
    
    function joinGame(gameRecord: Record) {
      dispatch("join-game", { gameHash: gameRecord.signed_action.hashed.hash });
    }
    
    function onGameCreated(event) {
      fetchAvailableGames();
      dispatch("join-game", { gameHash: event.detail.gameHash });
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
              <span>Game ID: {getGameId(gameRecord)}</span>
              <button on:click={() => joinGame(gameRecord)}>Join Game</button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
    
    <section class="create-game">
      <h3>Create a New Game</h3>
      <CreateGame on:game-created={onGameCreated} />
    </section>
    
    <section class="global-scoreboard">
      <h3>Global Scoreboard</h3>
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
    h2, h3 { text-align: center; }
    .game-list,
    .create-game,
    .global-scoreboard {
      margin-bottom: 2rem;
      background-color: #333;
      padding: 1rem;
      border-radius: 8px;
    }
    .game-list ul { list-style: none; padding: 0; }
    .game-list li {
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
    button:hover { background-color: #535bf2; }
    .error { color: #ff8080; text-align: center; }
  </style>
  