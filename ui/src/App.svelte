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
  import { playerProfile, checkAndLoadExistingProfile } from "./stores/playerProfile";
  import { currentGame } from "./stores/currentGame";
  // Import invitation store and helpers
  import { invitations, addInvitation, removeInvitation } from "./stores/invitationStore";
  import { getOrFetchProfile, cacheProfile, type DisplayProfile } from "./stores/profilesStore";
  // Import the specific signal type
  // MODIFIED: Added GlobalChatMessageSignal
  import type { GameInvitationSignal, GameStartedSignal, GlobalChatMessageSignal, GameAbandonedSignal } from "./ping_2_pong/ping_2_pong/types"; // Adjust path if necessary
  // Import chat store function
  import { addChatMessage } from "./stores/chatStore"; // Adjust path if necessary
  // Import utility functions
  import { truncatePubkey } from "./utils";
  // Import Holochain constants
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "./holochainConfig";

  // Import Components
  import WelcomePopup from "./ping_2_pong/WelcomePopup.svelte";
  // import GlobalChat from "./ping_2_pong/chat/GlobalChat.svelte"; // REMOVED
  import Dashboard from "./ping_2_pong/game/Dashboard.svelte";
  import type { Dashboard as DashboardType } from "./ping_2_pong/game/Dashboard.svelte"; // For instance binding
  import PongGame from "./ping_2_pong/game/PongGame.svelte";
  import StatisticsDashboard from "./ping_2_pong/game/StatisticsDashboard.svelte";
  import InvitationPopup from "./ping_2_pong/game/InvitationPopup.svelte"; // Adjust path if needed
  import OpponentLeftPopup from "./ping_2_pong/game/OpponentLeftPopup.svelte";

  // Define the UnsubscribeFunction type locally
  type UnsubscribeFunction = () => void;

  // Component State
  let client: AppClient;
  let error: HolochainError | undefined; // For critical/global errors
  let loading = true; // Global loading state
  let presenceIntervalId: ReturnType<typeof setInterval> | undefined;
  let unsubscribeFromSignals: UnsubscribeFunction | undefined; // Use the locally defined type
  let invitationError: string | null = null; // Specific for invitation errors
  let dashboardComponent: DashboardType; // Variable to hold Dashboard instance

  let showOpponentLeftPopup = false;
  let opponentWhoLeftNickname: string | null = null;
  let opponentWhoLeftAgentKeyB64: string | null = null;

  // Holochain Client Setup
  const appClientContext = {
    getClient: async (): Promise<AppClient> => {
      if (!client) {
        // console.log("Connecting to Holochain...");
        try {
          const urlParams = new URLSearchParams(window.location.search);
          const appPort = urlParams.get("app_port") || urlParams.get("port") || (window as any).__HC_PORT__;
          if (appPort) {
            client = await AppWebsocket.connect({ url: new URL(`ws://localhost:${appPort}`) });
          } else {
            client = await AppWebsocket.connect();
          }
          // console.log("Holochain client connected.");
        } catch (e) { console.error("AppWebsocket.connect error:", e); error = e as HolochainError; throw e; }
      }
      return client;
    }
  };

  let isMounted = true;

  // --- Presence Publishing ---
  async function publishPresence() {
      const regStatus = get(isRegistered);
      if (!isMounted || !client || !regStatus) return;
      try {
          await client.callZome({ cap_secret: null, role_name: HOLOCHAIN_ROLE_NAME, zome_name: HOLOCHAIN_ZOME_NAME, fn_name: "publish_presence", payload: null, });
      } catch(e) {
          if (!isMounted) return;
          if ((e as HolochainError).message.includes("source chain head has moved")) {
              console.warn("Presence publishing skipped due to source chain conflict (likely harmless).");
          } else {
             console.error("Error publishing presence from App.svelte:", e);
          }
      }
  }


  function unwrapSignal(raw: any): any {
    if (!raw) return null;
    let s = raw;
    if (s?.App?.payload) s = s.App.payload;
    if (s?.value?.payload) s = s.value.payload;
    if (s?.payload) s = s.payload;
    if (Array.isArray(s)) s = s[0];
    return s;
  }

  // --- Signal Handler ---
  function handleSignal(signalPayload: any) {
      const s = unwrapSignal(signalPayload);
      if (!s || !s.type) return;

      if (s.type === "PresenceUpdate") {
          if (s.agent_key && s.nickname) {
              cacheProfile(s.agent_key, s.nickname);
          }
      } else if (s.type === "GameInvitation") {
          const inviter = s.inviter || s.inviter_pub_key;
          const gameId = s.game_id;
          const message = s.message || "You have been invited to play Pong!";
          if (gameId && inviter) {
              if (encodeHashToBase64(inviter) !== encodeHashToBase64(client.myPubKey)) {
                  addInvitation({ game_id: gameId, inviter, message, type: "GameInvitation" });
              }
          }
      } else if (s.type === "GameStarted") {
          const { game_id, player_1, player_2 } = s;
          if (game_id && player_1 && player_2) {
              const myPubKeyB64 = encodeHashToBase64(client.myPubKey);
              const p1B64 = encodeHashToBase64(player_1);
              const p2B64 = encodeHashToBase64(player_2);
              if (myPubKeyB64 === p1B64 || myPubKeyB64 === p2B64) {
                  currentGame.set(game_id);
                  currentRoute.set("gameplay");
                  invitations.set([]);
              }
          }
      } else if (s.type === "GlobalChatMessage") {
          const sender = s.sender;
          const content = s.content;
          const timestamp = s.timestamp;

          let senderB64 = "";
          if (typeof sender === "string") {
              senderB64 = sender;
          } else if (sender) {
              try { senderB64 = encodeHashToBase64(sender); } catch(e) { senderB64 = String(sender); }
          }

          let messageTimestamp = Date.now();
          if (typeof timestamp === "number") {
              messageTimestamp = timestamp > 1e12 ? Math.floor(timestamp / 1000) : timestamp;
          } else if (Array.isArray(timestamp) && timestamp.length >= 1) {
              messageTimestamp = timestamp[0] * 1000 + Math.floor((timestamp[1] || 0) / 1000000);
          }

          if (content && typeof content === "string") {
              addChatMessage({
                  type: "GlobalChatMessage",
                  sender: senderB64,
                  content: content,
                  timestamp: messageTimestamp,
              });
          }
      } else if (s.type === "GameAbandoned") {
          const { game_id: abandonedGameId, abandoned_by_player } = s;
          const currentLocalGameId = get(currentGame);
          if (currentLocalGameId && abandonedGameId && encodeHashToBase64(abandonedGameId) === encodeHashToBase64(currentLocalGameId)) {
              getOrFetchProfile(client, abandoned_by_player).then(profile => {
                  if (profile && profile.nickname) {
                      opponentWhoLeftNickname = profile.nickname;
                  } else {
                      opponentWhoLeftNickname = truncatePubkey(abandoned_by_player);
                  }
                  opponentWhoLeftAgentKeyB64 = encodeHashToBase64(abandoned_by_player);
                  showOpponentLeftPopup = true;
              }).catch(() => {
                  opponentWhoLeftNickname = truncatePubkey(abandoned_by_player);
                  opponentWhoLeftAgentKeyB64 = encodeHashToBase64(abandoned_by_player);
                  showOpponentLeftPopup = true;
              });
              currentGame.set(null);
              currentRoute.set("dashboard");
              invitations.set([]);
          }
      }
  }


  // --- Event Handlers ---
  function handleJoinGame(event: CustomEvent<{ gameHash: ActionHash }>) {
    // This event is dispatched by Lobby/Popup after *calling* join_game.
    // Navigation now relies solely on receiving the GameStarted signal.
    // console.log("[App.svelte handleJoinGame] Event received, waiting for GameStarted signal.", event.detail); // Info
    invitations.set([]); // Still clear invitations if one was accepted
  }

  function handleRegistration() {
    // console.log('Player registered!'); // Info
  }

  // --- Popup Action Handlers ---
  async function handleAcceptInvitation(
    event: CustomEvent<{ gameId: string | ActionHash }>
  ) {
    const gameHash: ActionHash =
      typeof event.detail.gameId === "string"
        ? decodeHashFromBase64(event.detail.gameId)
        : event.detail.gameId;

    // console.log("[App] Accepting invitation for", encodeHashToBase64(gameHash)); // Info

    removeInvitation(gameHash);     // optimistic removal
    loading = true; // Use global loading for now, can refine later
    invitationError = null; // Clear previous error

    try {
      await client.callZome({
        cap_secret: null,
        role_name : HOLOCHAIN_ROLE_NAME,
        zome_name : HOLOCHAIN_ZOME_NAME,
        fn_name   : "accept_invitation",   /* ← new zome call */
        payload   : { game_id: gameHash }
      });

      // console.log("[App] accept_invitation sent – waiting for GameStarted…"); // Info
    } catch (e: any) {
      console.error("accept_invitation error:", e);
      invitationError = e.data?.data || e.message || "Failed to accept invitation.";
    } finally {
      loading = false;
    }
  }

  function handleDeclineInvitation(gameIdToDecline: ActionHash) {
      removeInvitation(gameIdToDecline);
      invitationError = null;
  }

  // --- Exit Game Handler ---
  function exitGame() {
      currentGame.set(null);
      currentRoute.set("dashboard");
      invitations.set([]);
      if (dashboardComponent && typeof dashboardComponent.refreshLeaderboardData === 'function') {
        setTimeout(() => {
          if (dashboardComponent && typeof dashboardComponent.refreshLeaderboardData === 'function') {
            dashboardComponent.refreshLeaderboardData();
          }
        }, 0);
      }
  }


  // --- Lifecycle Hooks ---
  onMount(async () => {
    try {
      loading = true;
      const fetchedClient = await appClientContext.getClient();
      if (!isMounted) return;
      client = fetchedClient;
      if (client) {
          unsubscribeFromSignals = client.on("signal", handleSignal);
          await checkAndLoadExistingProfile(client);
          if (!isMounted) return;
          await publishPresence();
          if (!isMounted) return;
      }
      presenceIntervalId = setInterval(publishPresence, 60000);
    } catch (e) { 
      if (!isMounted) return;
      console.error("Failed to initialize Holochain client or load profile:", e);
      error = e as HolochainError;
    }
    finally { 
      if (isMounted) loading = false; 
    }
  });

  onDestroy(() => {
      isMounted = false;
      if (unsubscribeFromSignals) { unsubscribeFromSignals(); /* console.log("App.svelte signal listener detached."); */ } // Info
      if (presenceIntervalId) { clearInterval(presenceIntervalId); }
      // console.log("App destroyed"); // Info
  });

  // Provide client context
  setContext(clientContext, appClientContext);

  // Reactive derivations
  const isRegistered = derived(playerProfile, ($p) => $p !== null);
  $: route = $currentRoute || 'dashboard';
  $: gameId = $currentGame;
  $: currentPlayerProfile = $playerProfile;
  $: currentInvitationToShow = $invitations.length > 0 ? $invitations[0] : null;

</script>

{#if loading} <main><p>Connecting to Holochain...</p></main>
{:else if error} <main> <p>Error Connecting: {error.message}</p> <p>Please ensure the Holochain conductor is running...</p> </main>
{:else if !$isRegistered}
  <WelcomePopup on:registered={handleRegistration} />
{:else}
  <main class="app-main">
    {#if currentPlayerProfile}
      <header class="user-header">
        <p><strong>Name:</strong> {currentPlayerProfile.nickname}</p>
        <p><strong>Agent Key:</strong> {truncatePubkey(currentPlayerProfile.agentKey)}</p>
      </header>
    {/if}

    <!-- GlobalChat component REMOVED from App.svelte -->

    {#if currentInvitationToShow}
       {@const inviterName = truncatePubkey(currentInvitationToShow.inviter)}
       {@const gameIdString = encodeHashToBase64(currentInvitationToShow.game_id)}
       {@const gameIdObject = currentInvitationToShow.game_id}

       <InvitationPopup
         inviter={inviterName}
         gameId={gameIdString}
         error={invitationError}
         on:accept={(e) => handleAcceptInvitation(e)}
         on:decline={() => handleDeclineInvitation(gameIdObject)}
       />
    {/if}

    {#if route === "dashboard"}
      <Dashboard on:join-game={handleJoinGame} bind:this={dashboardComponent} />
    {:else if route === "gameplay"}
       {#if currentPlayerProfile && gameId}
           <PongGame
             gameId={gameId}
             playerKey={client?.myPubKey || currentPlayerProfile.agentKeyB64}
             on:exit-game={exitGame}
           />
       {:else}
           <p>Loading game data or missing information...</p>
           <button on:click={exitGame}>Back to Dashboard</button>
       {/if}
    {:else if route === "statistics"}
      <StatisticsDashboard />
    {:else}
       <Dashboard on:join-game={handleJoinGame} bind:this={dashboardComponent} />
    {/if}

    {#if showOpponentLeftPopup && opponentWhoLeftNickname && opponentWhoLeftAgentKeyB64}
      <OpponentLeftPopup
        opponentNickname={opponentWhoLeftNickname}
        opponentAgentKeyB64={opponentWhoLeftAgentKeyB64}
        on:dismissed={() => showOpponentLeftPopup = false}
      />
    {/if}
  </main>
{/if}
