<script lang="ts">
  import { onMount, onDestroy, setContext } from "svelte";
  // Import Holochain client essentials
  import { AppWebsocket, encodeHashToBase64, decodeHashFromBase64 } from "@holochain/client";
  // Make sure ActionHash is imported if used in types
  import type { AppClient, HolochainError, ActionHash, AgentPubKey } from "@holochain/client";
  // Import Svelte helpers/stores
  import { derived, get } from "svelte/store"; // Import get from svelte/store
  import { clientContext } from "./contexts";
  import { currentRoute } from "./stores/routeStore";
  import { playerProfile } from "./stores/playerProfile";
  import { currentGame } from "./stores/currentGame";
  // Import invitation store and helpers
  import { invitations, addInvitation, removeInvitation } from "./stores/invitationStore";
  // Import the specific signal type
  import type { GameInvitationSignal, GameStartedSignal } from "./ping_2_pong/ping_2_pong/types"; // Adjust path if necessary

  // Import Components
  import WelcomePopup from "./ping_2_pong/WelcomePopup.svelte";
  import Dashboard from "./ping_2_pong/game/Dashboard.svelte";
  import PongGame from "./ping_2_pong/game/PongGame.svelte";
  import StatisticsDashboard from "./ping_2_pong/game/StatisticsDashboard.svelte";
  import InvitationPopup from "./ping_2_pong/game/InvitationPopup.svelte"; // Adjust path if needed

  // Define the UnsubscribeFunction type locally
  type UnsubscribeFunction = () => void;

  // Component State
  let client: AppClient;
  let error: HolochainError | undefined;
  let loading = true;
  let presenceIntervalId: ReturnType<typeof setInterval> | undefined;
  let unsubscribeFromSignals: UnsubscribeFunction | undefined; // Use the locally defined type

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
      const regStatus = get(isRegistered);
      if (!client || !regStatus) return;
      try {
          await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "publish_presence", payload: null, });
      } catch(e) {
          if ((e as HolochainError).message.includes("source chain head has moved")) {
              console.warn("Presence publishing skipped due to source chain conflict (likely harmless).");
          } else {
             console.error("Error publishing presence from App.svelte:", e);
          }
      }
  }


  // --- Signal Handler ---
  function handleSignal(signalPayload: any) {
      console.log("[App.svelte handleSignal] Received signal RAW:", JSON.stringify(signalPayload, null, 2));

      if (signalPayload && typeof signalPayload === 'object' && signalPayload.App) {
          const appSignalWrapper = signalPayload.App;
          console.log("[App.svelte handleSignal] Processing App signal wrapper:", appSignalWrapper);

          const actualSignal = appSignalWrapper.payload;

          if (!actualSignal?.type) {
              console.log("[App.svelte handleSignal] App signal payload ignored (missing type).", actualSignal);
              return;
          }

          console.log(`[App.svelte handleSignal] Detected signal type: ${actualSignal.type}`);

          // Handle GameInvitation signals
          if (actualSignal.type === "GameInvitation") {
              console.log("[App.svelte handleSignal] Processing GameInvitation...");
              const invitation = actualSignal as GameInvitationSignal;
              if (invitation.game_id && invitation.inviter && invitation.message) {
                  if (encodeHashToBase64(invitation.inviter) !== encodeHashToBase64(client.myPubKey)) {
                      console.log("[App.svelte handleSignal] Adding invitation to store:", invitation);
                      addInvitation(invitation);
                      console.log("[App.svelte handleSignal] Invitations store state:", get(invitations));
                  } else {
                      console.log("[App.svelte handleSignal] Ignoring self-sent GameInvitation signal.");
                  }
              } else {
                  console.warn("[App.svelte handleSignal] Malformed GameInvitation signal received:", invitation);
              }
          // Handle GameStarted signals
          } else if (actualSignal.type === "GameStarted") {
              console.log("[App.svelte handleSignal] Processing GameStarted...");
              const { game_id, player_1, player_2 } = actualSignal as GameStartedSignal;

              if (game_id && player_1 && player_2) {
                   const myPubKeyB64 = encodeHashToBase64(client.myPubKey);
                   const p1B64 = encodeHashToBase64(player_1);
                   const p2B64 = encodeHashToBase64(player_2);

                   // *** ADDED DETAILED LOGS ***
                   console.log(`[App.svelte handleSignal GameStarted] MyKey: ${myPubKeyB64}, P1: ${p1B64}, P2: ${p2B64}`);

                   // Check if I am one of the players in the started game
                   if (myPubKeyB64 === p1B64 || myPubKeyB64 === p2B64) {
                       console.log(`[App.svelte handleSignal GameStarted] Match found! I am involved.`);
                       console.log(`[App.svelte handleSignal GameStarted] Setting currentGame to: ${encodeHashToBase64(game_id)}`);
                       currentGame.set(game_id);
                       console.log(`[App.svelte handleSignal GameStarted] Setting currentRoute to: gameplay`);
                       currentRoute.set("gameplay");
                       console.log(`[App.svelte handleSignal GameStarted] Clearing invitations.`);
                       invitations.set([]);
                       console.log(`[App.svelte handleSignal GameStarted] Navigation logic complete.`);
                   } else {
                       console.log(`[App.svelte handleSignal GameStarted] Ignoring signal for game ${encodeHashToBase64(game_id)} as I am not a participant.`);
                   }
              } else {
                   console.warn("[App.svelte handleSignal GameStarted] Signal missing required fields (game_id, player_1, player_2)", actualSignal);
              }
          // Handle standard signals
          } else if (actualSignal.type === "EntryCreated") {
              console.log("[App.svelte handleSignal] Received EntryCreated signal (standard).");
          } else if (actualSignal.type === "LinkCreated") {
              console.log("[App.svelte handleSignal] Received LinkCreated signal (standard).");
          }
          else {
              console.log(`[App.svelte handleSignal] Received unhandled signal type in payload: ${actualSignal.type}`);
          }
      } else {
          console.log("[App.svelte handleSignal] Received signal ignored (not App signal structure):", signalPayload);
      }
  }


  // --- Event Handlers ---
  function handleJoinGame(event: CustomEvent<{ gameHash: ActionHash }>) {
    // This event is dispatched by Lobby/Popup after *calling* join_game.
    // Navigation now relies solely on receiving the GameStarted signal.
    console.log("[App.svelte handleJoinGame] Event received, waiting for GameStarted signal.", event.detail);
    invitations.set([]); // Still clear invitations if one was accepted
  }

  function handleRegistration() { console.log('Player registered!'); }

  // --- Popup Action Handlers ---
  async function handleAcceptInvitation(
    event: CustomEvent<{ gameId: string | ActionHash }>
  ) {
    const gameHash: ActionHash =
      typeof event.detail.gameId === "string"
        ? decodeHashFromBase64(event.detail.gameId)
        : event.detail.gameId;

    console.log("[App] Accepting invitation for", encodeHashToBase64(gameHash));

    removeInvitation(gameHash);     // optimistic removal
    loading = true;

    try {
      await client.callZome({
        cap_secret: null,
        role_name : "ping_2_pong",
        zome_name : "ping_2_pong",
        fn_name   : "accept_invitation",   /* ← new zome call */
        payload   : { game_id: gameHash }
      });

      /* Navigation now waits for the `GameStarted` signal */
      console.log("[App] accept_invitation sent – waiting for GameStarted…");
    } catch (e) {
      console.error("accept_invitation error:", e);
      error = e as HolochainError;
    } finally {
      loading = false;
    }
  }

  function handleDeclineInvitation(gameIdToDecline: ActionHash) {
      console.log("[App.svelte handleDeclineInvitation] Declining invitation for game:", encodeHashToBase64(gameIdToDecline));
      removeInvitation(gameIdToDecline);
  }

  // --- Exit Game Handler ---
  function exitGame() {
      console.log("[App.svelte exitGame] Exiting game...");
      // TODO: Optionally call backend to update game status (e.g., cancel/finish)
      currentGame.set(null);
      currentRoute.set("dashboard");
      invitations.set([]);
  }


  // --- Lifecycle Hooks ---
  onMount(async () => {
    try {
      loading = true;
      client = await appClientContext.getClient();
      if (client) {
          unsubscribeFromSignals = client.on("signal", handleSignal);
          console.log("App.svelte signal listener attached.");
      }
      presenceIntervalId = setInterval(publishPresence, 15000);
    } catch (e) { console.error("Failed to initialize Holochain client:", e); error = e as HolochainError;}
    finally { loading = false; }
  });

  onDestroy(() => {
      if (unsubscribeFromSignals) { unsubscribeFromSignals(); console.log("App.svelte signal listener detached."); }
      if (presenceIntervalId) { clearInterval(presenceIntervalId); }
      console.log("App destroyed");
  });

  // Provide client context
  setContext(clientContext, appClientContext);

  // Reactive derivations
  const isRegistered = derived(playerProfile, ($p) => $p !== null);
  let route: string; currentRoute.subscribe((value) => { route = value || 'dashboard'; });
  let gameId: ActionHash | null = null; currentGame.subscribe((value) => { gameId = value; });
  let currentPlayerProfile: { nickname: string; agentKey: AgentPubKey } | null; playerProfile.subscribe((value) => { currentPlayerProfile = value; });

  // Derive state for the current invitation popup
  let currentInvitationToShow: GameInvitationSignal | null = null;
  invitations.subscribe(invList => {
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
           <PongGame
             gameId={gameId}
             playerKey={currentPlayerProfile.agentKey}
             on:exit-game={exitGame}
           />
       {:else}
           <p>Loading game data or missing information...</p>
           <button on:click={exitGame}>Back to Dashboard</button>
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
