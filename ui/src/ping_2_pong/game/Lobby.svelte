<script lang="ts">
  import { onMount } from "svelte";
  import { getContext, createEventDispatcher } from "svelte";
  import type { AppClient, Record, HolochainError } from "@holochain/client";
  import { clientContext } from "../../contexts";
  import type { ClientContext } from "../../contexts";

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  let loading: boolean = false;
  let error: string | null = null;

  // This function first checks for any waiting game.
  // If one is found, it dispatches a join-game event.
  // If not, it creates a new game (always in Waiting state) and then dispatches the join event.
  async function joinOrCreateGame() {
    loading = true;
    error = null;
    try {
      // Fetch available games.
      const result = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_all_games",
        payload: null,
      });
      let waitingGame: Record | null = null;
      if (result) {
        const waitingGames = result.filter((r: Record) => {
          // Defensive check: ensure the record has a valid entry.
          const entry = (r.entry as any)?.Present?.entry;
          if (!entry || !entry.game_status) {
            console.log("Skipping record without valid game entry:", r);
            return false;
          }
          // We assume game_status is now an object with a "type" field.
          return entry.game_status.type === "Waiting";
        });
        if (waitingGames.length > 0) {
          waitingGame = waitingGames[0];
        }
      }

      if (waitingGame) {
        // If a waiting game exists, join it.
        dispatch("join-game", { gameHash: waitingGame.signed_action.hashed.hash });
      } else {
        // Otherwise, create a new game with the current agent as player_1.
        // Since the game is created in Waiting state, no need for player_2.
        const newGameRecord: Record = await client.callZome({
          cap_secret: null,
          role_name: "ping_2_pong",
          zome_name: "ping_2_pong",
          fn_name: "create_game",
          payload: {
            player_1: client.myPubKey,
            player_2: null,
            created_at: Date.now(),
            game_status: { type: "Waiting" },
            player_1_paddle: 250,
            player_2_paddle: 250,
            ball_x: 400,
            ball_y: 300,
          },
        });
        dispatch("join-game", { gameHash: newGameRecord.signed_action.hashed.hash });
      }
    } catch (e) {
      error = (e as HolochainError).message;
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    client = await appClientContext.getClient();
  });
</script>

<div class="lobby">
  {#if loading}
    <p>Loading...</p>
  {:else if error}
    <p class="error">Error: {error}</p>
  {:else}
    <button on:click={joinOrCreateGame}>Play</button>
  {/if}
</div>

<style>
  .lobby {
    padding: 1rem;
    text-align: center;
    color: #fff;
  }
  .error {
    color: #ff8080;
  }
  button {
    font-size: 1.5rem;
    padding: 1rem 2rem;
    border: none;
    background-color: #646cff;
    color: white;
    border-radius: 8px;
    cursor: pointer;
    transition: background-color 0.25s;
  }
  button:hover {
    background-color: #535bf2;
  }
</style>
