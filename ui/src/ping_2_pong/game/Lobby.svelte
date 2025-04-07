<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { getContext } from "svelte";
  import type { AppClient, Record, HolochainError, AgentPubKey, ActionHash } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  import { clientContext } from "../../contexts";
  import type { ClientContext } from "../../contexts";
  import type { PlayerStatus } from "../ping_2_pong/types"; // Assuming PlayerStatus enum is defined here or imported

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // State for joining/creating game.
  let loading: boolean = false;
  let error: string | null = null;

  // State for online users and their status
  interface OnlineUser {
    pubKey: AgentPubKey;
    status: PlayerStatus | 'Loading' | 'Error'; // Add status field
  }
  let onlineUsers: OnlineUser[] = [];
  // Removed outdated caching logic for simplicity now, can be added back if needed
  // let lastOnlineUsers: OnlineUser[] = [];
  // let lastOnlineUpdateTime: number = 0;
  let fetchingUsers: boolean = false;
  let fetchError: string | null = null;

  // For invitations.
  let invitationError: string | null = null;
  interface Invitation {
      game_id: ActionHash; // Use ActionHash type
      inviter: AgentPubKey;
      message: string;
  }
  let invitations: Invitation[] = [];

  // Helper: Convert and truncate an AgentPubKey.
  function truncatePubkey(pubkey: AgentPubKey): string {
    const base64 = encodeHashToBase64(pubkey);
    return base64.slice(0, 8) + "..." + base64.slice(-6);
  }

  // Function to join or create a random game. (Logic mostly unchanged)
  async function joinOrCreateGame() {
    // ... (keep existing joinOrCreateGame logic) ...
    // Find waiting game or create new one
    // Dispatch "join-game" with gameHash
    loading = true;
    error = null;
    try {
      const allGames: Record[] = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_all_games",
        payload: null,
      });

      let waitingGame: Record | null = null;
      if (allGames && Array.isArray(allGames)) {
          // Filter for games in 'Waiting' status and without player_2 set yet
          // (Assuming Game entry structure is accessible like this)
          waitingGame = allGames.find((r: Record) => {
              try {
                  // Need to safely access nested properties
                  const entry = (r.entry as any)?.Present?.entry;
                  return entry?.game_status?.type === "Waiting" && entry?.player_2 === null;
              } catch (e) {
                  console.warn("Could not parse game entry:", r, e);
                  return false;
              }
          }) || null; // Find returns undefined if not found, convert to null
      }

      if (waitingGame) {
        console.log("Joining waiting game:", waitingGame.signed_action.hashed.hash);
        // TODO: Need to call an 'accept_game' or similar zome fn
        //       to update the game entry with player_2 = myPubKey
        //       and potentially change status to InProgress.
        // For now, just dispatching join:
        dispatch("join-game", { gameHash: waitingGame.signed_action.hashed.hash });
      } else {
         console.log("No waiting games found, creating new game...");
        const newGameRecord: Record = await client.callZome({
          cap_secret: null,
          role_name: "ping_2_pong",
          zome_name: "ping_2_pong",
          fn_name: "create_game",
           // Payload minimal, defaults set in coordinator now
          payload: {
            player_1: client.myPubKey,
            player_2: null, // Explicitly null for waiting game
            // created_at, game_status, paddle/ball positions set by coordinator
          },
        });
        console.log("Created new game:", newGameRecord.signed_action.hashed.hash);
        dispatch("join-game", { gameHash: newGameRecord.signed_action.hashed.hash });
      }
    } catch (e) {
      console.error("Error joining/creating game:", e);
      error = (e as HolochainError).message;
    } finally {
      loading = false;
    }
  }

  // Function to send an invitation (Logic mostly unchanged)
  async function sendInvitation(invitee: AgentPubKey) {
      invitationError = null;
      // Should not invite someone already in game - Button will be disabled
    try {
      console.log("Creating game for invitation to:", encodeHashToBase64(invitee));
      const gameRecord: Record = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "create_game",
        // Payload sets both players, status defaults to Waiting
        payload: {
            player_1: client.myPubKey,
            player_2: invitee,
        }
      });
      const gameHash = gameRecord.signed_action.hashed.hash;
      console.log("Game created for invitation:", gameHash);

      console.log("Sending invitation signal...");
      // Use the Invitation type defined in game.rs coordinator
      const invitationPayload: Invitation = {
        game_id: gameHash,
        inviter: client.myPubKey,
        message: "You have been invited to play Pong!",
      };
      // Use the Signal enum structure for clarity if possible,
      // otherwise send the raw payload if send_signal expects SerializedBytes directly.
      // Assuming send_signal takes SerializedBytes:
      const signalPayload = { // This structure needs to match what the receiver expects
         type: "GameInvitation", // Add type field for receiver filtering
         ...invitationPayload
      }
      await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "send_signal",
        payload: signalPayload, // Send the structured payload
      });
      console.log("Invitation signal sent.");

      // Join the game immediately for the inviter
      dispatch("join-game", { gameHash: gameHash });
    } catch (e) {
      console.error("Error sending invitation:", e);
      invitationError = (e as HolochainError).message;
    }
  }

  // Publish presence (Unchanged)
  async function publishPresence() {
    try {
      const result = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "publish_presence",
        payload: null,
      });
      // console.log("Presence published:", result); // Less noisy log
    } catch (e) {
      console.error("Error publishing presence:", e);
    }
  }

  // Fetch online users AND their status
  async function fetchOnlineUsersAndStatus() {
    if (fetchingUsers) return; // Prevent concurrent fetches
    fetchingUsers = true;
    fetchError = null;
    try {
      const fetchedPubKeys: AgentPubKey[] = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_online_users",
        payload: null,
      });

      // Create initial user list with 'Loading' status
      const users = fetchedPubKeys.map(pubKey => ({ pubKey, status: 'Loading' } as OnlineUser));
      onlineUsers = users; // Update UI immediately with loading state

      // Fetch status for each user individually (can be slow, consider batching if possible)
      for (let i = 0; i < onlineUsers.length; i++) {
          const user = onlineUsers[i];
          try {
                const statusResult: PlayerStatus = await client.callZome({
                    cap_secret: null,
                    role_name: "ping_2_pong",
                    zome_name: "ping_2_pong",
                    fn_name: "get_player_status",
                    payload: user.pubKey,
                });
                // Update status in place - Svelte reactivity needs assignment
                onlineUsers[i] = { ...user, status: statusResult };
          } catch (statusError) {
                console.error(`Error fetching status for ${truncatePubkey(user.pubKey)}:`, statusError);
                onlineUsers[i] = { ...user, status: 'Error' };
          }
      }
       // Trigger Svelte reactivity by re-assigning the array
       onlineUsers = [...onlineUsers];

    } catch (e) {
      console.error("Error fetching online users:", e);
      fetchError = (e as HolochainError).message;
      onlineUsers = []; // Clear users on fetch error
    } finally {
      fetchingUsers = false;
    }
  }

  // Signal handling for invitations (Unchanged)
  function subscribeToInvitations() {
    return client.on("signal", (signalPayload: any) => {
        // Ensure signal structure matches what's sent
        if (signalPayload.type === "GameInvitation") {
            // Ensure required fields exist
            if (!signalPayload.game_id || !signalPayload.inviter || !signalPayload.message) {
                 console.warn("Received malformed GameInvitation signal:", signalPayload);
                 return;
            }
            // Check if signal is from self
             if (encodeHashToBase64(signalPayload.inviter) !== encodeHashToBase64(client.myPubKey)) {
                console.log("Received invitation signal:", signalPayload);
                // Add the invitation to the local state if not already present
                 if (!invitations.some(inv => encodeHashToBase64(inv.game_id) === encodeHashToBase64(signalPayload.game_id))) {
                     invitations = [...invitations, {
                        game_id: signalPayload.game_id, // Should already be ActionHash
                        inviter: signalPayload.inviter, // Should already be AgentPubKey
                        message: signalPayload.message
                     }];
                 }
             }
        } else {
            // Optional: log other signals for debugging
            // console.debug("Received other signal:", signalPayload);
        }
    });
  }

  let presenceInterval: ReturnType<typeof setInterval>;
  let onlineInterval: ReturnType<typeof setInterval>;
  let invitationUnsub: (() => void) | undefined; // Store the unsubscribe function

  onMount(async () => {
    client = await appClientContext.getClient();
    await publishPresence();
    await fetchOnlineUsersAndStatus(); // Initial fetch includes status now
    presenceInterval = setInterval(publishPresence, 15000); // every 15 sec
    onlineInterval = setInterval(fetchOnlineUsersAndStatus, 10000); // every 10 sec
    invitationUnsub = client.on("signal", subscribeToInvitations()); // Store unsub function
  });

  onDestroy(() => {
    clearInterval(presenceInterval);
    clearInterval(onlineInterval);
    if (invitationUnsub) invitationUnsub(); // Call the unsubscribe function
  });
</script>

<div class="lobby">
  <section class="online-users">
    <h2>Online Users</h2>
    {#if fetchingUsers && onlineUsers.length === 0}
      <p>Loading online users...</p>
    {:else if fetchError}
      <p class="error">Error fetching users: {fetchError}</p>
    {:else if onlineUsers.length === 0}
      <p>No users online</p>
    {:else}
      <ul>
        {#each onlineUsers as user (encodeHashToBase64(user.pubKey))}
          {#if encodeHashToBase64(user.pubKey) !== encodeHashToBase64(client.myPubKey)}
            {@const isDisabled = !(typeof user.status === 'object' && user.status !== null && user.status.type === 'Available')}
            <li>
              <span>
                {truncatePubkey(user.pubKey)}
                {#if user.status === 'Loading'}
                  <em class="status">(Checking...)</em>
                {:else if user.status === 'Error'}
                  <em class="status error">(Status Error)</em>
                {:else if typeof user.status === 'object' && user.status !== null && user.status.type === 'InGame'}
                  <em class="status">(In Game)</em>
                {:else if typeof user.status === 'object' && user.status !== null && user.status.type === 'Available'}
                   <em class="status available">(Available)</em>
                {:else}
                   <em class="status">({typeof user.status === 'object' ? JSON.stringify(user.status) : user.status})</em>
                {/if}
              </span>
              <button
                on:click={() => sendInvitation(user.pubKey)}
                disabled={isDisabled}
                class:disabled={isDisabled}
              >
                Invite
              </button>
            </li>
          {/if}
        {/each}
      </ul>
    {/if}
    {#if invitationError}
       <p class="error">Invitation Error: {invitationError}</p>
    {/if}
  </section>

  <section class="play-button">
    {#if loading}
      <p>Joining/Creating Game...</p>
    {:else if error}
      <p class="error">Error: {error}</p>
    {:else}
      <button on:click={joinOrCreateGame}>Play Random</button>
    {/if}
  </section>

  {#if invitations.length > 0}
    <section class="invitations">
      <h2>Game Invitations</h2>
      <ul>
        {#each invitations as inv, index (encodeHashToBase64(inv.game_id))}
          <li>
            <p>
              <strong>From:</strong> {truncatePubkey(inv.inviter)}<br/>
              <em>{inv.message}</em>
            </p>
            <button on:click={() => {
                 dispatch("join-game", { gameHash: inv.game_id });
                 // Remove accepted invitation
                 invitations = invitations.filter((_, i) => i !== index);
            }}>
              Accept
            </button>
            <button on:click={() => invitations = invitations.filter((_, i) => i !== index)}>
              Decline
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</div>

<style>
  .lobby {
    padding: 1rem;
    text-align: center;
    color: #fff;
    display: flex;
    flex-direction: column;
    gap: 1.5rem; /* Slightly reduced gap */
  }
  .online-users {
    margin: 0; /* Removed top/bottom margin */
    padding: 1rem;
    background-color: #3a3a3a; /* Slightly lighter background */
    border-radius: 8px;
    color: #e0e0e0; /* Lighter text */
  }
  .online-users h2 {
      margin-top: 0;
      color: orange; /* Keep title orange */
      font-weight: bold;
  }
  .online-users ul {
    list-style: none;
    padding: 0;
    margin: 0;
    max-height: 200px; /* Limit height */
    overflow-y: auto; /* Add scroll */
  }
  .online-users li {
    margin: 0.6rem 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.4rem; /* Add padding */
    border-bottom: 1px solid #555; /* Separator */
  }
   .online-users li:last-child {
       border-bottom: none;
   }
  .error {
    color: #ff8080;
    font-size: 0.9em;
  }
  .status {
      font-size: 0.85em;
      margin-left: 0.5em;
      color: #aaa;
  }
  .status.available {
      color: lightgreen;
  }
  .status.error {
      color: #ff8080;
  }
  button {
    font-size: 1rem; /* Smaller button text */
    padding: 0.4rem 0.8rem; /* Smaller padding */
    border: none;
    background-color: #646cff;
    color: white;
    border-radius: 6px; /* Slightly smaller radius */
    cursor: pointer;
    transition: background-color 0.25s;
  }
  button:hover {
    background-color: #535bf2;
  }
  button.disabled {
      background-color: #555;
      cursor: not-allowed;
      opacity: 0.6;
  }
  .play-button button {
      font-size: 1.5rem; /* Keep play random button larger */
      padding: 0.8rem 1.5rem;
  }
  .invitations {
    margin: 1rem;
    padding: 1rem;
    background-color: #2a2a2a; /* Darker background */
    border: 1px solid #646cff;
    border-radius: 8px;
    color: #fff;
  }
  .invitations h2 {
      margin-top: 0;
      color: orange;
  }
  .invitations ul { list-style: none; padding: 0; margin: 0; }
  .invitations li {
    margin: 0.7rem 0;
    border-bottom: 1px solid #444;
    padding-bottom: 0.7rem;
    display: flex; /* Align items */
    justify-content: space-between;
    align-items: center;
  }
   .invitations li p {
       margin: 0;
       text-align: left;
   }
   .invitations li div { /* Container for buttons */
       display: flex;
       gap: 0.5rem;
   }
</style>