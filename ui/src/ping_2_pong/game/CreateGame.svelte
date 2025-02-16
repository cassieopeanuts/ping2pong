<script lang="ts">
  import type { ActionHash, AgentPubKey, AppClient, HolochainError, Record } from "@holochain/client";
  import { createEventDispatcher, getContext, onMount } from "svelte";
  import { type ClientContext, clientContext } from "../../contexts";
  import type { Game, GameStatus } from "../ping_2_pong/types";

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // For game creation, only one agent (the host) creates the game.
  let player1: AgentPubKey;
  let player2: AgentPubKey | null; // Leave as null initially.

  // Use the current time for the creation timestamp.
  export let createdAt: number = Date.now();

  // Default game status.
  let gameStatus: GameStatus = { type: "Waiting" };

  $: isGameValid = true;

  onMount(async () => {
    client = await appClientContext.getClient();
    // For game creation, assign player1 to the current agent's key.
    player1 = client.myPubKey;
    // Leave player2 as null since no second agent is joining yet.
    player2 = null;
  });

  async function createGame() {
    // Build the game entry without a game_id (it is computed in the DNA).
    const gameEntry = {
      player_1: player1,
      player_2: player2,
      created_at: createdAt,
      game_status: gameStatus,
      player_1_paddle: 250,
      player_2_paddle: 250,
      ball_x: 400,
      ball_y: 300,
    } as Omit<Game, "game_id">;

    try {
      const record: Record = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "create_game",
        payload: gameEntry,
      });
      // Use the returned entry hash as the game's unique identifier.
      dispatch("game-created", { gameHash: record.signed_action.hashed.hash });
    } catch (e) {
      alert((e as HolochainError).message);
    }
  }
</script>

<div>
  <h3>Create Game</h3>
  <div>
    <label for="Game Status">Game Status:</label>
    <select name="Game Status" bind:value={gameStatus.type}>
      <option value="Waiting">Waiting</option>
      <option value="InProgress">In Progress</option>
      <option value="Finished">Finished</option>
    </select>
  </div>
  <button disabled={!isGameValid} on:click={createGame}>
    Create Game
  </button>
</div>
