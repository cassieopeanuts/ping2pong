<script lang="ts">
  import type { ActionHash, AgentPubKey, AppClient, HolochainError, Record } from "@holochain/client";
  import { createEventDispatcher, getContext, onMount } from "svelte";
  import { clientContext, type ClientContext } from "../../contexts";
  import type { Game, GameStatus } from "../ping_2_pong/types";

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // For game creation, only one agent (the host) creates the game.
  let player1: AgentPubKey;
  // Leave player2 undefined if no opponent is available.
  let player2: AgentPubKey | undefined = undefined;

  // Use the current time for the creation timestamp.
  export let createdAt: number = Date.now();

  // New games are always created in Waiting state.
  let gameStatus: GameStatus = { type: "Waiting" };

  $: isGameValid = true;

  onMount(async () => {
    client = await appClientContext.getClient();
    // Use the current agent's public key as player1.
    player1 = client.myPubKey;
    // player2 remains undefined
  });

  async function createGame() {
    // Build the game entry. We conditionally add player2 only if defined.
    const gameEntry = {
      player_1: player1,
      created_at: createdAt,
      game_status: gameStatus,
      player_1_paddle: 250,
      player_2_paddle: 250,
      ball_x: 400,
      ball_y: 300,
      ...(player2 ? { player2 } : {})  // omit player_2 if not set
    } as Omit<Game, "game_id">;

    try {
      const record: Record = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "create_game",
        payload: gameEntry,
      });
      dispatch("game-created", { gameHash: record.signed_action.hashed.hash });
    } catch (e) {
      alert((e as HolochainError).message);
    }
  }
</script>

<div>
  <h3>Create Game</h3>
  <button disabled={!isGameValid} on:click={createGame}>
    Create Game
  </button>
</div>

<style>
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
