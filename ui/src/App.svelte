<script lang="ts">
  import { onMount, onDestroy, setContext } from "svelte";
  import { AppWebsocket, encodeHashToBase64, decodeHashFromBase64 } from "@holochain/client";
  import type { AppClient, HolochainError, ActionHash, AgentPubKey } from "@holochain/client";
  import { derived, get } from "svelte/store"; 
  import { clientContext } from "./contexts";
  import { currentRoute } from "./stores/routeStore";
  import { playerProfile, checkAndLoadExistingProfile } from "./stores/playerProfile";
  import { currentGame } from "./stores/currentGame";
  import { invitations, addInvitation, removeInvitation } from "./stores/invitationStore";
  import { getOrFetchProfile, type DisplayProfile } from "./stores/profilesStore";
  import type { GameInvitationSignal, GameStartedSignal, GlobalChatMessageSignal, GameAbandonedSignal } from "./ping_2_pong/ping_2_pong/types"; // Adjust path if necessary
  import { addChatMessage } from "./stores/chatStore"; 
  import { truncatePubkey } from "./utils";
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "./holochainConfig";

  import WelcomePopup from "./ping_2_pong/WelcomePopup.svelte";
  import Dashboard from "./ping_2_pong/game/Dashboard.svelte";
  import type { Dashboard as DashboardType } from "./ping_2_pong/game/Dashboard.svelte"; 
  import PongGame from "./ping_2_pong/game/PongGame.svelte";
  import StatisticsDashboard from "./ping_2_pong/game/StatisticsDashboard.svelte";
  import InvitationPopup from "./ping_2_pong/game/InvitationPopup.svelte"; 
  import OpponentLeftPopup from "./ping_2_pong/game/OpponentLeftPopup.svelte";

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
        try {
          client = await AppWebsocket.connect({ url: new URL("ws://localhost:8888") });
          // console.log("Holochain client connected.");
        } catch (e) { console.error("AppWebsocket.connect error:", e); error = e as HolochainError; throw e; }
      }
      return client;
    }
  };

  // --- Presence Publishing ---
  async function publishPresence() {
      const regStatus = get(isRegistered);
      if (!client || !regStatus) return;
      try {
          await client.callZome({ cap_secret: null, role_name: HOLOCHAIN_ROLE_NAME, zome_name: HOLOCHAIN_ZOME_NAME, fn_name: "publish_presence", payload: null, });
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
      console.log("%%%% RAW SIGNAL RECEIVED BY CLIENT:", JSON.stringify(signalPayload, null, 2));
      // console.log("[App.svelte handleSignal] Received signal RAW:", JSON.stringify(signalPayload, null, 2)); // Too verbose

      if (signalPayload && typeof signalPayload === 'object' && signalPayload.App) {
          const appSignalWrapper = signalPayload.App;
          // console.log("[App.svelte handleSignal] Processing App signal wrapper:", appSignalWrapper); // Verbose

          const actualSignal = appSignalWrapper.payload;

          if (!actualSignal?.type) {
              // console.log("[App.svelte handleSignal] App signal payload ignored (missing type).", actualSignal); // Can be noisy
              return;
          }

          // console.log(`[App.svelte handleSignal] Detected signal type: ${actualSignal.type}`); // Info

          // Handle GameInvitation signals
          if (actualSignal.type === "GameInvitation") {
              // console.log("[App.svelte handleSignal] Processing GameInvitation...");
              const invitation = actualSignal as GameInvitationSignal;
              if (invitation.game_id && invitation.inviter && invitation.message) {
                  if (encodeHashToBase64(invitation.inviter) !== encodeHashToBase64(client.myPubKey)) {
                      // console.log("[App.svelte handleSignal] Adding invitation to store:", invitation); // Info
                      addInvitation(invitation);
                      // console.log("[App.svelte handleSignal] Invitations store state:", get(invitations)); // Debug
                  } else {
                      // console.log("[App.svelte handleSignal] Ignoring self-sent GameInvitation signal."); // Info
                  }
              } else {
                  console.warn("[App.svelte handleSignal] Malformed GameInvitation signal received:", invitation);
              }
          // Handle GameStarted signals
          } else if (actualSignal.type === "GameStarted") {
              // console.log("[App.svelte handleSignal] Processing GameStarted...");
              const { game_id, player_1, player_2 } = actualSignal as GameStartedSignal;

              if (game_id && player_1 && player_2) {
                   const myPubKeyB64 = encodeHashToBase64(client.myPubKey);
                   const p1B64 = encodeHashToBase64(player_1);
                   const p2B64 = encodeHashToBase64(player_2);

                   // console.log(`[App.svelte handleSignal GameStarted] MyKey: ${myPubKeyB64}, P1: ${p1B64}, P2: ${p2B64}`); // Debug

                   // Check if I am one of the players in the started game
                   if (myPubKeyB64 === p1B64 || myPubKeyB64 === p2B64) {
                       // console.log(`[App.svelte handleSignal GameStarted] Match found! I am involved.`);
                       // console.log(`[App.svelte handleSignal GameStarted] Setting currentGame to: ${encodeHashToBase64(game_id)}`);
                       currentGame.set(game_id);
                       // console.log(`[App.svelte handleSignal GameStarted] Setting currentRoute to: gameplay`);
                       currentRoute.set("gameplay");
                       // console.log(`[App.svelte handleSignal GameStarted] Clearing invitations.`);
                       invitations.set([]);
                       // console.log(`[App.svelte handleSignal GameStarted] Navigation logic complete.`);
                   } else {
                       // console.log(`[App.svelte handleSignal GameStarted] Ignoring signal for game ${encodeHashToBase64(game_id)} as I am not a participant.`); // Info
                   }
              } else {
                   console.warn("[App.svelte handleSignal GameStarted] Signal missing required fields (game_id, player_1, player_2)", actualSignal);
              }
          // Handle standard signals
          } else if (actualSignal.type === "EntryCreated") {
              // console.log("[App.svelte handleSignal] Received EntryCreated signal (standard)."); // Info
          } else if (actualSignal.type === "LinkCreated") {
              // console.log("[App.svelte handleSignal] Received LinkCreated signal (standard)."); // Info
          }
          // MODIFIED: Added GlobalChatMessage handler
          else if (actualSignal.type === "GlobalChatMessage") {
            // console.log("[App.svelte handleSignal] Processing GlobalChatMessage..."); // Kept for specific debugging if needed
            const rawSignal = actualSignal as any; // Keep 'as any' for flexibility

            // Check for the numeric timestamp first
            if (rawSignal.sender && typeof rawSignal.content === 'string' && typeof rawSignal.timestamp === 'number') {

                const messageTimestamp = Math.floor(rawSignal.timestamp / 1000); 
                const senderB64 = encodeHashToBase64(rawSignal.sender); 

                const chatSignal: GlobalChatMessageSignal = {
                    type: "GlobalChatMessage",
                    sender: senderB64,    // Use encoded sender
                    content: rawSignal.content,
                    timestamp: messageTimestamp, // Converted to milliseconds
                };
                addChatMessage(chatSignal);
                // console.log("[App.svelte handleSignal] Added chat message to store (numeric timestamp, encoded sender):", chatSignal); // Info

            // Fallback for original [seconds, nanoseconds] array format (optional, but good for robustness)
            } else if (rawSignal.sender && typeof rawSignal.content === 'string' &&
                Array.isArray(rawSignal.timestamp) && rawSignal.timestamp.length === 2 &&
                typeof rawSignal.timestamp[0] === 'number' && typeof rawSignal.timestamp[1] === 'number') {
                
                const messageTimestamp = rawSignal.timestamp[0] * 1000 + Math.floor(rawSignal.timestamp[1] / 1000000);
                const senderB64 = encodeHashToBase64(rawSignal.sender); // Encode sender
                
                const chatSignal: GlobalChatMessageSignal = {
                    type: "GlobalChatMessage",
                    sender: senderB64, // Use encoded sender
                    content: rawSignal.content,
                    timestamp: messageTimestamp,
                };
                addChatMessage(chatSignal);
                // console.log("[App.svelte handleSignal] Added chat message to store (array timestamp, encoded sender):", chatSignal); // Info
            }
            else {
                // If neither matches, then it's malformed
                console.warn("[App.svelte handleSignal] Malformed GlobalChatMessage signal received or sender/timestamp issue (unhandled format):", rawSignal);
            }
          } else if (actualSignal.type === "GameAbandoned") {
              console.log("[App.svelte handleSignal] Processing GameAbandoned signal:", actualSignal);
              const { game_id: abandonedGameId, abandoned_by_player } = actualSignal as GameAbandonedSignal; // Cast to the new type

              const currentLocalGameId = get(currentGame);
              if (currentLocalGameId && abandonedGameId && encodeHashToBase64(abandonedGameId) === encodeHashToBase64(currentLocalGameId)) {
                  console.log(`[App.svelte handleSignal] Current game ${encodeHashToBase64(currentLocalGameId)} was abandoned by ${encodeHashToBase64(abandoned_by_player)}. Showing popup.`);
            
                  // Fetch profile of the player who abandoned
                  getOrFetchProfile(client, abandoned_by_player).then(profile => {
                      if (profile && profile.nickname) {
                          opponentWhoLeftNickname = profile.nickname;
                      } else {
                          opponentWhoLeftNickname = truncatePubkey(abandoned_by_player); // Fallback to truncated pubkey
                      }
                      opponentWhoLeftAgentKeyB64 = encodeHashToBase64(abandoned_by_player);
                      showOpponentLeftPopup = true;
                  }).catch(profileError => {
                      console.error("[App.svelte handleSignal] Error fetching profile for opponent who left:", profileError);
                      opponentWhoLeftNickname = truncatePubkey(abandoned_by_player); // Fallback
                      opponentWhoLeftAgentKeyB64 = encodeHashToBase64(abandoned_by_player);
                      showOpponentLeftPopup = true;
                  });

                  currentGame.set(null);
                  currentRoute.set("dashboard");
                  invitations.set([]); 
              } else {
                  // This log is important for debugging signals for other games or if self-abandonment signal is received
                  console.log(`[App.svelte handleSignal] Received GameAbandoned signal for game ${abandonedGameId ? encodeHashToBase64(abandonedGameId) : 'unknown'}, but it does not match current game ${currentLocalGameId ? encodeHashToBase64(currentLocalGameId) : 'none'}. Or I was the one who abandoned (UI handles this separately).`);
              }
          }
          else {
              console.warn(`[App.svelte handleSignal] Received unhandled signal type in payload: ${actualSignal.type}`, actualSignal);
          }
      } else {
          // console.log("[App.svelte handleSignal] Received signal ignored (not App signal structure):", signalPayload); // Can be noisy
      }
  }


  // --- Event Handlers ---
  function handleJoinGame(event: CustomEvent<{ gameHash: ActionHash }>) {
    // This event is dispatched by Lobby/Popup after *calling* join_game.
    // console.log("[App.svelte handleJoinGame] Event received, waiting for GameStarted signal.", event.detail); // Info
    invitations.set([]); 
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

    removeInvitation(gameHash);     
    loading = true; 
    invitationError = null; 

    try {
      await client.callZome({
        cap_secret: null,
        role_name : HOLOCHAIN_ROLE_NAME,
        zome_name : HOLOCHAIN_ZOME_NAME,
        fn_name   : "accept_invitation",   
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
      // console.log("[App.svelte handleDeclineInvitation] Declining invitation for game:", encodeHashToBase64(gameIdToDecline)); // Info
      removeInvitation(gameIdToDecline);
      invitationError = null; 
  }

  // --- Exit Game Handler ---
  function exitGame() {
      // console.log("[App.svelte exitGame] Exiting game..."); // Info
      currentGame.set(null);
      currentRoute.set("dashboard");
      invitations.set([]); 

      // Add this block to refresh leaderboard
      if (dashboardComponent && typeof dashboardComponent.refreshLeaderboardData === 'function') {
        // Use setTimeout to allow Svelte to render/update the dashboard component first
        setTimeout(() => {
          // Double check instance, as component might be destroyed if route change was too fast or something else happened
          if (dashboardComponent && typeof dashboardComponent.refreshLeaderboardData === 'function') {
            console.log("[App.svelte] exitGame: Attempting to refresh leaderboard data via Dashboard component.");
            dashboardComponent.refreshLeaderboardData();
          } else {
            console.warn("[App.svelte] exitGame: Dashboard component or refreshLeaderboardData method not available after timeout.");
          }
        }, 0);
      } else {
        // This can happen if the Dashboard component wasn't rendered yet or `bind:this` hasn't updated `dashboardComponent`
        console.warn("[App.svelte] exitGame: Dashboard component not immediately available for refresh request. This might be okay if the dashboard is about to be mounted.");
      }
  }


  // --- Lifecycle Hooks ---
  onMount(async () => {
    try {
      loading = true;
      client = await appClientContext.getClient();
      if (client) {
          unsubscribeFromSignals = client.on("signal", handleSignal);
          // console.log("App.svelte signal listener attached."); // Info
          await checkAndLoadExistingProfile(client);
      }
      presenceIntervalId = setInterval(publishPresence, 15000);
    } catch (e) { 
      console.error("Failed to initialize Holochain client or load profile:", e); 
      error = e as HolochainError;
    }
    finally { 
      loading = false; 
    }
  });

  onDestroy(() => {
      if (unsubscribeFromSignals) { unsubscribeFromSignals(); /* console.log("App.svelte signal listener detached."); */ } // Info
      if (presenceIntervalId) { clearInterval(presenceIntervalId); }
      // console.log("App destroyed"); // Info
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
      if (invList.length > 0) {
        if (!currentInvitationToShow || encodeHashToBase64(currentInvitationToShow.game_id) !== encodeHashToBase64(invList[0].game_id)) {
          invitationError = null; // Clear error when a new invitation appears
        }
        currentInvitationToShow = invList[0];
      } else {
        currentInvitationToShow = null;
        invitationError = null; // Clear error if no invitations are present
      }
  });

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
       <Dashboard on:join-game={handleJoinGame} bind:this={dashboardComponent} />
       {() => { if (route !== 'dashboard') { console.warn(`Unknown route: ${route}, defaulting.`); setTimeout(() => currentRoute.set('dashboard'), 0); } return ''; }}
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
