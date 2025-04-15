<script lang="ts">
  import { onMount, onDestroy, setContext } from "svelte";
  // Import Holochain client essentials
  import { AppWebsocket, encodeHashToBase64 } from "@holochain/client";
  import type { AppClient, HolochainError, ActionHash, AgentPubKey } from "@holochain/client";
  // Import Svelte helpers/stores
  import { derived } from "svelte/store";
  import { clientContext } from "./contexts";
  import { currentRoute } from "./stores/routeStore";
  import { playerProfile } from "./stores/playerProfile";
  import { currentGame } from "./stores/currentGame";
  // *** Import invitation store and helpers ***
  import { invitations, addInvitation, removeInvitation } from "./stores/invitationStore";
  import type { GameInvitationSignal } from "./ping_2_pong/ping_2_pong/types";

  // Import Components
  import WelcomePopup from "./ping_2_pong/WelcomePopup.svelte";
  import Dashboard from "./ping_2_pong/game/Dashboard.svelte";
  import PongGame from "./ping_2_pong/game/PongGame.svelte";
  import StatisticsDashboard from "./ping_2_pong/game/StatisticsDashboard.svelte";
  // *** Import Invitation Popup ***
  import InvitationPopup from "./ping_2_pong/game/InvitationPopup.svelte"; // Adjust path if needed

  type UnsubscribeFunction = () => void;
  

  // Component State
  let client: AppClient;
  let error: HolochainError | undefined;
  let loading = true;
  let presenceIntervalId: ReturnType<typeof setInterval> | undefined;
  let unsubscribeFromSignals: UnsubscribeFunction | undefined;

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


  // --- Signal Handler ---
  function handleSignal(signalPayload: any) {
      console.log("App.svelte received signal:", signalPayload);

      // Ensure payload and type exist
      if (!signalPayload?.type) return;

      // Check if the payload is a GameInvitation
      if (signalPayload.type === "GameInvitation") {
          // Basic validation of the invitation structure
          if (signalPayload.game_id && signalPayload.inviter && signalPayload.message) {
              // Ensure we don't show invites sent by ourselves
              if (encodeHashToBase64(signalPayload.inviter) !== encodeHashToBase64(client.myPubKey)) {
                  console.log("Adding invitation to store:", signalPayload);
                  // Add to the global store (will automatically prevent duplicates)
                  addInvitation(signalPayload as GameInvitationSignal);
              } else {
                  console.log("Ignoring self-sent GameInvitation signal.");
              }
          } else {
              console.warn("Malformed GameInvitation signal received:", signalPayload);
          }
      } else if (signalPayload.type === "GameStarted") {
          console.log("Received GameStarted signal:", signalPayload);
          const { game_id, opponent } = signalPayload;
          if (game_id && opponent) {
              // Assume if we receive this, we are Player 1
               console.log(`Game ${encodeHashToBase64(game_id)} started with opponent ${encodeHashToBase64(opponent)}`);
               // Navigate P1 to the game
               currentGame.set(game_id);
               currentRoute.set("gameplay");
               // Clear any pending invitations when a game starts
               invitations.set([]);
          } else {
               console.warn("GameStarted signal missing game_id or opponent", signalPayload);
          }
      }
      // Add handlers for other signal types if needed
  }


  // --- Event Handlers ---

  // Handles join-game event from Dashboard/Lobby (when clicking "Play Random" or accepting invite)
  function handleJoinGame(event: CustomEvent<{ gameHash: ActionHash }>) {
    console.log("App.svelte handling join-game event:", event.detail);
    if (event.detail.gameHash) {
        currentGame.set(event.detail.gameHash);
        currentRoute.set("gameplay");
        // Clear any pending invitations when joining/starting a game
        invitations.set([]);
    } else {
        console.error("join-game event missing gameHash");
    }
  }

  // Handles registration from WelcomePopup (keep as is)
  function handleRegistration() { /* ... */ }

  // --- Popup Action Handlers ---
  async function handleAcceptInvitation(event: CustomEvent<{ gameId: ActionHash }>) {
      const gameIdToJoin = event.detail.gameId;
      console.log("Accepting invitation for game:", encodeHashToBase64(gameIdToJoin));
      // Remove the invitation from the store
      removeInvitation(gameIdToJoin);
      // Call the backend join_game function
      try {
          loading = true; // Optional: show loading indicator
          if (!client) throw new Error("Client not ready");
          // We don't need the full game record back here, just need to join
          await client.callZome({
              cap_secret: null,
              role_name: "ping_2_pong",
              zome_name: "ping_2_pong",
              fn_name: "join_game",
              payload: gameIdToJoin,
          });
          console.log("Successfully joined game via invitation accept.");
          // Navigate to the game screen
          currentGame.set(gameIdToJoin);
          currentRoute.set("gameplay");
      } catch(e) {
           console.error("Error joining game after accepting invitation:", e);
           error = e as HolochainError; // Show error to user
           // Maybe add the invitation back if join fails? Or show error message.
      } finally {
          loading = false;
      }
  }

  function handleDeclineInvitation(gameIdToDecline: ActionHash) {
    // const gameIdToDecline = event.detail.gameId; // Remove this line
    console.log("Declining invitation for game:", encodeHashToBase64(gameIdToDecline));
    // Remove the invitation from the store
    removeInvitation(gameIdToDecline);
}

  // --- Lifecycle Hooks ---
  onMount(async () => {
    try {
      loading = true;
      client = await appClientContext.getClient();
      // Start signal listener
      if (client) {
          unsubscribeFromSignals = client.on("signal", handleSignal);
          console.log("App.svelte signal listener attached.");
      }
      // Start presence interval
      presenceIntervalId = setInterval(publishPresence, 15000);
    } catch (e) { console.error("Failed to initialize Holochain client:", e); error = e as HolochainError;}
    finally { loading = false; }
  });

  onDestroy(() => {
      if (unsubscribeFromSignals) {
          unsubscribeFromSignals();
          console.log("App.svelte signal listener detached.");
      }
      if (presenceIntervalId) { clearInterval(presenceIntervalId); }
      console.log("App destroyed, closing Holochain connection.");
      // Consider closing client connection if appropriate: client?.close();
  });

  setContext(clientContext, appClientContext);

  const isRegistered = derived(playerProfile, ($p) => $p !== null);
  let route: string; currentRoute.subscribe((value) => { route = value || 'dashboard'; });
  let gameId: ActionHash | null = null; currentGame.subscribe((value) => { gameId = value; });
  let currentPlayerProfile: { nickname: string; agentKey: AgentPubKey } | null; playerProfile.subscribe((value) => { currentPlayerProfile = value; });

  // *** Derive state for the current invitation popup ***
  let currentInvitationToShow: GameInvitationSignal | null = null;
  invitations.subscribe(invList => {
      // Show the first invitation in the list
      currentInvitationToShow = invList.length > 0 ? invList[0] : null;
  });


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

    {#if currentInvitationToShow}
       {@const inviterName = truncatePubkey(currentInvitationToShow.inviter)}
       {@const gameIdString = encodeHashToBase64(currentInvitationToShow.game_id)}
       {@const gameIdObject = currentInvitationToShow.game_id}

       <InvitationPopup
       inviter={inviterName}
       gameId={gameIdString}
       on:accept={(e) => handleAcceptInvitation(e)}
       on:decline={() => handleDeclineInvitation(gameIdObject)} 
     />
    {/if}


    {#if route === "dashboard"}
      <Dashboard on:join-game={handleJoinGame} />
    {:else if route === "gameplay"}
       {#if currentPlayerProfile?.agentKey && gameId}
           <PongGame gameId={gameId} playerKey={currentPlayerProfile.agentKey} />
       {:else}
           <p>Loading game data...</p>
           <button on:click={() => { currentGame.set(null); currentRoute.set('dashboard'); invitations.set([]); }}>Back to Dashboard</button>
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