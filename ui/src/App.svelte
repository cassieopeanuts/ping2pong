<script lang="ts">
  import { onMount, setContext } from "svelte";
  import { AppWebsocket } from "@holochain/client";
  import { derived } from "svelte/store";
  import { clientContext } from "./contexts";
  import { currentRoute } from "./stores/routeStore";
  import { playerProfile } from "./stores/playerProfile";

  import WelcomePopup from "./ping_2_pong/WelcomePopup.svelte";
  import Dashboard from "./ping_2_pong/game/Dashboard.svelte";
  import PongGame from "./ping_2_pong/game/PongGame.svelte";
  import StatisticsDashboard from "./ping_2_pong/game/StatisticsDashboard.svelte";

  import type { AppClient, HolochainError, ActionHash } from "@holochain/client";
  import type { Game, GameStatus } from "./ping_2_pong/ping_2_pong/types";

  let client: AppClient;
  let error: HolochainError | undefined;
  let loading = false;

  const appClientContext = {
    getClient: async () => {
      if (!client) {
        client = await AppWebsocket.connect({ url: new URL("ws://localhost:8888") });
      }
      return client;
    }
  };

  onMount(async () => {
    try {
      loading = true;
      client = await appClientContext.getClient();
    } catch (e) {
      error = e as HolochainError;
    } finally {
      loading = false;
    }
  });

  setContext(clientContext, appClientContext);

  const isRegistered = derived(playerProfile, ($playerProfile) => $playerProfile !== null);

  let route: string;
  currentRoute.subscribe((value) => {
    route = value;
  });

  // Create a dummy game for routing purposes. In a real app you’d select a waiting game from the lobby.
  const waitingStatus: GameStatus = { type: "Waiting" };
  const dummyGame: Game = {
    game_id: "dummy_game_hash" as unknown as ActionHash,
    player_1: "dummy_player_1" as any,
    player_2: "dummy_player_2" as any,
    created_at: Date.now(),
    game_status: waitingStatus,
    player_1_paddle: 250,
    player_2_paddle: 250,
    ball_x: 400,
    ball_y: 300,
  };

  let currentPlayerProfile;
  playerProfile.subscribe((value) => {
    currentPlayerProfile = value;
  });
</script>

{#if loading}
  <main>
    <p>Connecting to Holochain...</p>
  </main>
{:else if error}
  <main>
    <p>Error: {error.message}</p>
  </main>
{:else}
  {#if !$isRegistered}
    <WelcomePopup />
  {:else}
    {#if route === "dashboard"}
      <Dashboard />
    {:else if route === "gameplay"}
      {#if currentPlayerProfile}
        <PongGame game={dummyGame} playerKey={currentPlayerProfile.agentKey} />
      {:else}
        <p>Loading game...</p>
      {/if}
    {:else if route === "statistics"}
      <StatisticsDashboard />
    {:else}
      <Dashboard />
    {/if}
  {/if}
{/if}

<style>
  main {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    font-family: sans-serif;
    color: #fff;
    background-color: #222;
  }
</style>
