<script lang="ts">
  import { onMount, onDestroy, setContext } from "svelte";
  // Import Holochain client essentials
  import { AppWebsocket, encodeHashToBase64 } from "@holochain/client"; // Removed decodeHashFromBase64 (unused)
  import type { AppClient, HolochainError, ActionHash, AgentPubKey } from "@holochain/client";
  // Import Svelte helpers/stores
  import { derived } from "svelte/store";
  import { clientContext } from "./contexts";
  import { currentRoute } from "./stores/routeStore";
  import { playerProfile } from "./stores/playerProfile";
  import { currentGame } from "./stores/currentGame"; // Now expects ActionHash | null

  // Import Components
  import WelcomePopup from "./ping_2_pong/WelcomePopup.svelte";
  import Dashboard from "./ping_2_pong/game/Dashboard.svelte";
  import PongGame from "./ping_2_pong/game/PongGame.svelte";
  import StatisticsDashboard from "./ping_2_pong/game/StatisticsDashboard.svelte";

  // Component State
  let client: AppClient;
  let error: HolochainError | undefined;
  let loading = true;
  let presenceIntervalId: ReturnType<typeof setInterval> | undefined;

  // Holochain Client Setup
  const appClientContext = {
    getClient: async (): Promise<AppClient> => {
      if (!client) {
        console.log("Connecting to Holochain...");
        try {
          client = await AppWebsocket.connect({ url: new URL("ws://localhost:8888") });
          console.log("Holochain client connected.");
        } catch (e) { console.error("AppWebsocket.connect error:", e); error = e as HolochainError; throw e; }
      }
      return client;
    }
  };

  // --- Helper Function ---
  function truncatePubkey(pubkey: AgentPubKey | null | undefined): string {
    if (!pubkey) return "N/A";
    try { const base64 = encodeHashToBase64(pubkey); return base64.slice(0, 8) + "..." + base64.slice(-6); }
    catch (e) { console.error("Error encoding pubkey in truncatePubkey:", e, pubkey); return "Error"; }
  }

  // --- Presence Publishing ---

  async function publishPresence() {
      // This check prevents publishing before client is ready AND user is registered
      if (!client || !$isRegistered) return;
      try {
          // console.log("Publishing presence...");
          await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "publish_presence", payload: null, });
      } catch(e) {
          // Ignore source chain errors in interval? Or maybe stop interval?
          if ((e as HolochainError).message.includes("source chain head has moved")) {
              console.warn("Presence publishing skipped due to source chain conflict (likely harmless).");
          } else {
             console.error("Error publishing presence from App.svelte:", e);
          }
      }
  }

  onMount(async () => {
    try {
      loading = true;
      client = await appClientContext.getClient();
      // Start the interval once client is ready.
      // The publishPresence function itself checks $isRegistered.
      presenceIntervalId = setInterval(publishPresence, 15000);
      // FIX: Remove initial publishPresence call here, let the interval handle it
      // if(client && $isRegistered) {
      //     await publishPresence();
      // }
    } catch (e) { console.error("Failed to initialize Holochain client:", e); error = e as HolochainError;}
    finally { loading = false; }
  });

  onDestroy(() => {
      if (presenceIntervalId) { clearInterval(presenceIntervalId); }
      console.log("App destroyed, closing Holochain connection.");
  });

  setContext(clientContext, appClientContext);

  const isRegistered = derived(playerProfile, ($p) => $p !== null);
  let route: string; currentRoute.subscribe((value) => { route = value || 'dashboard'; });
  let gameId: ActionHash | null = null; currentGame.subscribe((value) => { gameId = value; });
  let currentPlayerProfile: { nickname: string; agentKey: AgentPubKey } | null; playerProfile.subscribe((value) => { currentPlayerProfile = value; });

  function handleJoinGame(event: CustomEvent<{ gameHash: ActionHash }>) { /* ... as before ... */ }

  // FIX: Remove the publishPresence call from here
  function handleRegistration() {
      console.log('Player registered!');
      // Let the already running interval handle the first publish.
      // if (client) { publishPresence(); } // REMOVED
  }

</script>

{#if loading} <main><p>Connecting to Holochain...</p></main>
{:else if error} <main> <p>Error Connecting: {error.message}</p> <p>Please ensure the Holochain conductor is running...</p> </main>
{:else if !$isRegistered}
  <WelcomePopup on:registered={handleRegistration} />
{:else}
  <main>
    {#if currentPlayerProfile}
      <header class="user-header">
        <p><strong>Name:</strong> {currentPlayerProfile.nickname}</p>
        <p><strong>Agent Key:</strong> {truncatePubkey(currentPlayerProfile.agentKey)}</p>
      </header>
    {/if}

    {#if route === "dashboard"}
      <Dashboard on:join-game={handleJoinGame} />
    {:else if route === "gameplay"}
       {#if currentPlayerProfile?.agentKey && gameId}
           <PongGame gameId={gameId} playerKey={currentPlayerProfile.agentKey} />
       {:else}
           <p>Loading game data...</p>
           <button on:click={() => { currentGame.set(null); currentRoute.set('dashboard'); }}>Back to Dashboard</button>
       {/if}
    {:else if route === "statistics"}
      <StatisticsDashboard />
    {:else}
       <Dashboard on:join-game={handleJoinGame} />
       {() => { if (route !== 'dashboard') { console.warn(`Unknown route: ${route}, defaulting.`); setTimeout(() => currentRoute.set('dashboard'), 0); } return ''; }}
    {/if}
  </main>
{/if}

<style>
  main { display: flex; flex-direction: column; align-items: center; justify-content: flex-start; min-height: 100vh; font-family: sans-serif; color: #fff; background-color: #222; padding-top: 10px; }
  .user-header { color: orange; padding: 0.5rem 1rem; text-align: center; width: 100%; background-color: #333; margin-bottom: 1rem; border-radius: 5px; }
  .user-header p { margin: 0.3rem 0; font-size: 0.9em; }
  .user-header p strong { color: #ddd; }
</style>