<script lang="ts">
  import { onMount, getContext } from "svelte";
  import type { AppClient, Record, HolochainError, Game } from "@holochain/client"; // AgentPubKey and ActionHash implicitly used via Game
  import { clientContext, type ClientContext } from "../../contexts";
  import { decode } from "@msgpack/msgpack";
  import { encodeHashToBase64 } from "@holochain/client"; // Used in console.log, good for debugging
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "../../holochainConfig";

  let loading: boolean = false;
  let statusMessage: string | null = null;
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  onMount(async () => {
    client = await appClientContext.getClient();
  });

  async function joinOrCreateGame() {
    loading = true;
    statusMessage = null;
    // invitationStatus = null; // Not present in PlayButton context

    if (!client) {
      statusMessage = "Holochain client not ready.";
      loading = false;
      return;
    }

    try {
      const allGames: Record[] = await client.callZome({
        cap_secret: null,
        role_name: HOLOCHAIN_ROLE_NAME,
        zome_name: HOLOCHAIN_ZOME_NAME,
        fn_name: "get_all_games",
        payload: null
      });

      // console.log(`[PlayButton] Found ${allGames.length} total games.`); // Optional: keep for debugging

      let joinableGame: Record | null = null;
      let myWaitingGame: Record | null = null;

      for (const original of allGames) {
        const latest: Record = await client.callZome({
          cap_secret: null,
          role_name: HOLOCHAIN_ROLE_NAME,
          zome_name: HOLOCHAIN_ZOME_NAME,
          fn_name: "get_latest_game",
          payload: original.signed_action.hashed.hash
        });

        const decoded = decode((latest.entry as any).Present.entry) as Game;

        const waitingAndOpen =
          decoded.game_status === "Waiting" && decoded.player_2 === null;

        const isMine =
          encodeHashToBase64(decoded.player_1) ===
          encodeHashToBase64(client.myPubKey);

        if (waitingAndOpen && !isMine) {
          joinableGame = original;
          break;
        }

        if (waitingAndOpen && isMine) {
          myWaitingGame = original;
        }
      }

      if (joinableGame) {
        await client.callZome({
          cap_secret: null,
          role_name: HOLOCHAIN_ROLE_NAME,
          zome_name: HOLOCHAIN_ZOME_NAME,
          fn_name: "join_game",
          payload: joinableGame.signed_action.hashed.hash
        });
        statusMessage = "Joining game… waiting for confirmation.";
      } else if (myWaitingGame) {
        statusMessage = "Already waiting for an opponent in your game.";
      } else {
        const record: Record = await client.callZome({
          cap_secret: null,
          role_name: HOLOCHAIN_ROLE_NAME,
          zome_name: HOLOCHAIN_ZOME_NAME,
          fn_name: "create_game",
          payload: { player_1: client.myPubKey, player_2: null }
        });
        // console.log( // Optional: keep for debugging
        //   "[PlayButton] Created new game, waiting:",
        //   encodeHashToBase64(record.signed_action.hashed.hash)
        // );
        statusMessage = "Game created. Waiting for an opponent…";
      }
    } catch (e) {
      console.error("Error in joinOrCreateGame:", e);
      statusMessage = (e as HolochainError).message || "An unknown error occurred.";
    } finally {
      loading = false;
    }
  }
</script>

<div class="play-random-button-container">
  {#if loading}
    <p class="loading-message">Joining/Creating Game...</p>
  {:else if statusMessage}
    <p class:error-message={!statusMessage.startsWith("Game created") && !statusMessage.startsWith("Joining game")} style="margin-top: 0.5rem; margin-bottom: 0.5rem;">
      {statusMessage}
    </p>
    <!-- Optional: Add a button to clear statusMessage or retry -->
    <button on:click={() => {statusMessage = null;}} class="button" style="margin-top: 0.5rem; font-size: 0.8em; padding: 0.4em 0.8em;">Try Again / Clear</button>
  {:else}
    <button on:click={joinOrCreateGame} class="button">
      Play Random
    </button>
  {/if}
</div>

<style>
  .play-random-button-container {
    text-align: center;
    padding: 1rem;
    display: flex; /* Use flex to center content vertically and horizontally */
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100px; /* Give it some minimum height to avoid layout jumps */
  }

  .play-random-button-container button.button {
    /* Rely on global .button styles from index.css for theme consistency */
    /* Add any specific sizing or minor adjustments here if needed */
    font-size: 1.5em; /* Example: Make the main Play Random button larger */
    padding: 0.8em 1.5em;
  }

  /* Loading and error messages will use global styles from index.css */
  /* (.loading-message and .error-message) */
  /* Ensure paragraph for statusMessage has some margin if it's not an error-message */
  .play-random-button-container p:not(.error-message):not(.loading-message) {
    color: var(--text-muted-color); /* Use a muted color for info messages */
    font-size: 0.9em;
  }
</style>
