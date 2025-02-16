<script lang="ts">
  import { onMount, setContext } from "svelte";
  import { AppWebsocket } from "@holochain/client";
  import { derived } from "svelte/store";
  import { clientContext } from "./contexts";
  import { currentRoute } from "./stores/routeStore";
  import { playerProfile } from "./stores/playerProfile";
  import { currentGame } from "./stores/currentGame";

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

  // currentGame store holds a string game hash.
  let gameId: string | null = null;
  currentGame.subscribe((value) => {
    gameId = value;
  });

  // Reactive variable to cast gameId to ActionHash (if available)
  $: castedGameId = gameId ? (gameId as unknown as ActionHash) : undefined;

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
      <!-- When a join-game event is dispatched, update currentGame and route -->
      <Dashboard on:join-game={(e) => {
        currentGame.set(e.detail.gameHash);
        currentRoute.set("gameplay");
      }} />
    {:else if route === "gameplay"}
      {#if currentPlayerProfile && castedGameId}
        <PongGame gameId={castedGameId} playerKey={currentPlayerProfile.agentKey} />
      {:else}
        <p>Loading game...</p>
      {/if}
    {:else if route === "statistics"}
      <StatisticsDashboard />
    {:else}
      <Dashboard on:join-game={(e) => {
        currentGame.set(e.detail.gameHash);
        currentRoute.set("gameplay");
      }} />
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
