<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { getContext } from "svelte";
  // Import types from holochain/client
  import type { AppClient, Record, HolochainError, AgentPubKey, ActionHash, Entry } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  // Import local context and types
  import { clientContext } from "../../contexts";
  import type { ClientContext } from "../../contexts";
  import type { PlayerStatus, GameInvitationSignal, Game } from "../ping_2_pong/types";

  import { decode } from "@msgpack/msgpack";

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // --- Component State ---
  let loading: boolean = false; // For Join/Create button
  let error: string | null = null;  // Error during Join/Create

  // Online Users State
  interface OnlineUser {
    pubKey: AgentPubKey;
    status: PlayerStatus | 'Loading' | 'Error'; // Status from zome or temporary state
  }
  let onlineUsers: OnlineUser[] = [];
  let fetchingUsers: boolean = false; // To prevent concurrent fetches
  let fetchError: string | null = null; // Error fetching users/status

  // Invitations State
  let invitationError: string | null = null; // Error sending invite
  let invitations: GameInvitationSignal[] = []; // Store received signals matching the type

  // --- Helper Functions ---
  function truncatePubkey(pubkey: AgentPubKey): string {
    try {
        const base64 = encodeHashToBase64(pubkey);
        return base64.slice(0, 8) + "..." + base64.slice(-6);
    } catch(e) {
        console.error("Error encoding pubkey:", e);
        return "Error";
    }
  }

  // --- Zome Calls & Logic ---

  // Executed when "Play Random" is clicked
  async function joinOrCreateGame() {
    loading = true; error = null;
    if (!client) { error = "Holochain client not ready."; loading = false; return; }
    try {
      console.log("Looking for waiting games...");
      const allGames: Record[] = await client.callZome({
        cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
        fn_name: "get_all_games", payload: null
      });
      console.log(`Found ${allGames ? allGames.length : 0} total games.`); // Log how many games were fetched

      let waitingGame: Record | null = null;
      if (allGames && Array.isArray(allGames)) {
        
      waitingGame = allGames.find((r: Record) => {
          // --- Add Detailed Logging Here ---
          console.log("--- Filtering Game Record ---");
          console.log("Record Action Hash:", encodeHashToBase64(r.signed_action.hashed.hash));
          console.log("Record Entry Structure:", r.entry);
          // -----------------------------------

          let actualEntry: Entry | undefined = undefined;
          const recordEntry = r.entry;

          // Check potential entry structures
          if (recordEntry && typeof recordEntry === 'object' && 'Present' in recordEntry && (recordEntry as any).Present) {
              actualEntry = (recordEntry as { Present: Entry }).Present;
              console.log("Extracted entry via 'Present'");
          }
          // Add other checks if necessary, e.g. if entry might be direct:
          // else if (recordEntry && ...) { actualEntry = recordEntry as Entry; console.log("Used entry directly"); }


              // FIX: Deserialize the extracted Entry before using it
              if (actualEntry && actualEntry.entry_type === 'App') { // Check it's an App entry containing bytes
                  try {
                      // Decode the msgpack bytes (actualEntry.entry is typically Uint8Array)
                      const decodedGame = decode(actualEntry.entry) as Game; // Cast the decoded object

                      // FIX: Compare game_status directly with string "Waiting"
                      const isWaiting = decodedGame.game_status === "Waiting";
                      const p2IsNull = decodedGame.player_2 === null;
                      const p1Exists = !!decodedGame.player_1;
                      const notMyGame = p1Exists && encodeHashToBase64(decodedGame.player_1) !== encodeHashToBase64(client.myPubKey);

                      console.log("Deserialized Game:", decodedGame); // Log the object
                      console.log("Filter Conditions:", { isWaiting, p2IsNull, notMyGame });
                      console.log("Game Status:", decodedGame.game_status);
                      console.log("Player 2:", decodedGame.player_2);
                      console.log("Player 1 Check:", { p1Exists, p1Hash: p1Exists? encodeHashToBase64(decodedGame.player_1) : 'N/A', myHash: encodeHashToBase64(client.myPubKey) });

                      return isWaiting && p2IsNull && notMyGame; // Return result
                  } catch (decodeError) {
                      console.error("Msgpack decoding error during filter:", decodeError, actualEntry);
                      return false; // Cannot process if decoding fails
                  }
              } else {
                  console.log(`Record ${encodeHashToBase64(r.signed_action.hashed.hash)} has no present App entry or unexpected structure.`);
                  return false;
              }
          }) || null;
      }

      // --- Logic for joining vs creating ---
      if (waitingGame && waitingGame.signed_action?.hashed?.hash) {
        // Found a waiting game -> JOIN IT
        const gameToJoinHash: ActionHash = waitingGame.signed_action.hashed.hash;
        console.log("Found waiting game, attempting to join:", encodeHashToBase64(gameToJoinHash));
        try {
            await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "join_game", payload: gameToJoinHash });
            console.log("Successfully joined game.");
            dispatch("join-game", { gameHash: gameToJoinHash });
        } catch (joinError) { console.error("Error joining game:", joinError); error = `Failed to join game: ${(joinError as HolochainError).message}`; }
      } else {
         // No suitable waiting game -> CREATE A NEW ONE
         console.log("No suitable waiting games found, creating new game...");
         const createPayload = { player_1: client.myPubKey, player_2: null };
         const newGameRecord: Record = await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "create_game", payload: createPayload });
         const newGameHash: ActionHash = newGameRecord.signed_action.hashed.hash;
         console.log("Created new game:", encodeHashToBase64(newGameHash));
         dispatch("join-game", { gameHash: newGameHash });
      }
    } catch (e) { console.error("Error in joinOrCreateGame:", e); error = (e as HolochainError).message; }
    finally { loading = false; }
  }

  // Executed when "Invite" button is clicked

  async function sendInvitation(invitee: AgentPubKey) {
    invitationError = null;
    if (!client) { invitationError = "Holochain client not ready."; return; }
  try {
    // 1. Create the game entry (keep this part)
    console.log("Creating game for invitation to:", encodeHashToBase64(invitee));
    const createPayload = { player_1: client.myPubKey, player_2: invitee };
    const gameRecord: Record = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "create_game",
        payload: createPayload
    });
    const gameHash: ActionHash = gameRecord.signed_action.hashed.hash;
    console.log("Game created for invitation:", encodeHashToBase64(gameHash)); // Log encoded hash

    // 2. Prepare the invitation payload (matches backend Invitation struct)
    // Explicitly type payload for clarity (optional but good practice)
    const invitationPayload: {
        game_id: ActionHash;
        inviter: AgentPubKey;
        message: string;
    } = {
      game_id: gameHash,
      inviter: client.myPubKey,
      message: "You have been invited to play Pong!",
    };

    // 3. Send the invitation using the correct function and payload
    console.log("Sending invitation...");
    await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "send_invitation",   // <<<--- FIX: Use the correct function name
      payload: invitationPayload    // <<<--- FIX: Send the Invitation object directly
    });
    console.log("Invitation sent.");

    // 4. Inviter joins the game immediately (keep this part)
    dispatch("join-game", { gameHash: gameHash });

  } catch (e) {
      console.error("Error sending invitation:", e);
      // Check if error has 'data' property for more backend info
      const errorData = (e as any)?.data?.data;
      invitationError = errorData ? `${(e as Error).message}: ${errorData}` : (e as Error).message;
  }
}

  // Periodically publish presence
  // MOVED TO App.svelte
  // async function publishPresence() {
  //   if (!client) return;
  //   try { await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "publish_presence", payload: null }); }
  //   catch (e) { console.error("Error publishing presence:", e); }
  // }

  // Periodically fetch online users and their game status
  async function fetchOnlineUsersAndStatus() {
    if (fetchingUsers || !client) return;
    fetchingUsers = true; fetchError = null;
    try {
      // Get online pubkeys
      const fetchedPubKeys: AgentPubKey[] = await client.callZome({
          cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
          fn_name: "get_online_users", payload: null
        });

      // Set initial state with 'Loading'
      const users = fetchedPubKeys.map(pubKey => ({ pubKey, status: 'Loading' } as OnlineUser));
      onlineUsers = users;

      // Fetch status for each user
      // Note: Batching this would be more efficient if backend supported it
      for (let i = 0; i < onlineUsers.length; i++) {
            const user = onlineUsers[i];
            try {
                  // Fetch status - expecting 'Available' or 'InGame' string directly now
                  const statusResult = await client.callZome({
                      cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
                      fn_name: "get_player_status", payload: user.pubKey
                  });
                  // Assign the result directly (it should be PlayerStatus string or handle if object)
                  if (typeof statusResult === 'string') {
                      onlineUsers[i] = { ...user, status: statusResult as PlayerStatus };
                  } else if (typeof statusResult === 'object' && statusResult !== null && 'type' in statusResult) {
                      // Handle if backend returns { type: 'Variant' } despite expectation
                      onlineUsers[i] = { ...user, status: (statusResult as any).type as PlayerStatus };
                  } else {
                       console.warn("Unexpected status result format:", statusResult);
                       onlineUsers[i] = { ...user, status: 'Error' };
                  }
            } catch (statusError) { console.error(`Error fetching status for ${truncatePubkey(user.pubKey)}:`, statusError); onlineUsers[i] = { ...user, status: 'Error' }; }
        }
      // Trigger Svelte reactivity
      onlineUsers = [...onlineUsers];

    } catch (e) {
        const errorMsg = (e as HolochainError).message;
        console.error("Error fetching online users:", errorMsg);
        // FIX: Handle source chain conflict gracefully
        if (errorMsg.includes("source chain head has moved")) {
            console.warn("Skipping online users update due to source chain conflict.");
            // Keep stale data instead of clearing: onlineUsers = [];
        } else {
            // For other errors, clear the list and show error
            fetchError = errorMsg;
            onlineUsers = [];
        }
    } finally {
        fetchingUsers = false;
    }
  }

  // Callback function for handling incoming signals
  function handleSignal(signalPayload: any) {
      if (signalPayload?.type === "GameInvitation") {
          const invitation = signalPayload as GameInvitationSignal; // Cast for type safety
          if (!invitation.game_id || !invitation.inviter || !invitation.message) { console.warn("Malformed invite:", signalPayload); return; }
          if (encodeHashToBase64(invitation.inviter) !== encodeHashToBase64(client.myPubKey)) {
              if (!invitations.some(inv => encodeHashToBase64(inv.game_id) === encodeHashToBase64(invitation.game_id))) {
                   invitations = [...invitations, invitation]; // Store the full signal object
                   console.log("Added invitation:", invitation);
              }
          }
      }
  }

  // --- Lifecycle ---
  // Presence moved to APp
  // let presenceInterval: ReturnType<typeof setInterval>;
  let onlineInterval: ReturnType<typeof setInterval>;
  let invitationUnsub: (() => void) | undefined;

  onMount(async () => {
    client = await appClientContext.getClient();
    await fetchOnlineUsersAndStatus(); // Initial fetch
    // FIX: Stagger intervals slightly
    onlineInterval = setInterval(fetchOnlineUsersAndStatus, 11000); // e.g., 11 seconds
    // REMOVED presence interval (now in App.svelte, e.g., every 17 seconds)
    invitationUnsub = client.on("signal", handleSignal);
  });

  onDestroy(() => {
    clearInterval(onlineInterval);
    if (invitationUnsub) invitationUnsub();
  });

</script>

<div class="lobby">
  <section class="online-users">
    <h2>Online Users</h2>
    {#if fetchingUsers && onlineUsers.length === 0} <p>Loading online users...</p>
    {:else if fetchError} <p class="error">Error fetching users: {fetchError}</p>
    {:else if onlineUsers.filter(u => encodeHashToBase64(u.pubKey) !== encodeHashToBase64(client?.myPubKey)).length === 0}
      <p>No other users online</p>
    {:else}
      <ul>
        {#each onlineUsers as user (encodeHashToBase64(user.pubKey))}
          {#if encodeHashToBase64(user.pubKey) !== encodeHashToBase64(client?.myPubKey)}
            {@const isDisabled = !(user.status === 'Available')}
            <li>
              <span>
                {truncatePubkey(user.pubKey)}
                {#if user.status === 'Loading'} <em class="status">(Checking...)</em>
                {:else if user.status === 'Error'} <em class="status error">(Status Error)</em>
                {:else if user.status === 'InGame'} <em class="status">(In Game)</em>
                {:else if user.status === 'Available'} <em class="status available">(Available)</em>
                {:else} <em class="status">(Unknown)</em> {/if}
              </span>
              <button on:click={() => sendInvitation(user.pubKey)} disabled={isDisabled} class:disabled={isDisabled}> Invite </button>
            </li>
          {/if}
        {/each}
      </ul>
    {/if}
    {#if invitationError} <p class="error" style="margin-top: 10px;">Invitation Error: {invitationError}</p> {/if}
  </section>

  <section class="play-button">
    {#if loading} <p>Joining/Creating Game...</p>
    {:else if error} <p class="error">Error: {error}</p>
    {:else} <button on:click={joinOrCreateGame}>Play Random</button> {/if}
  </section>

  {#if invitations.length > 0}
    <section class="invitations">
      <h2>Game Invitations</h2>
      <ul>
        {#each invitations as inv, index (encodeHashToBase64(inv.game_id))}
          <li>
             <p> <strong>From:</strong> {truncatePubkey(inv.inviter)}<br/> <em>{inv.message}</em> </p>
             <div> <button on:click={() => { dispatch("join-game", { gameHash: inv.game_id }); invitations = invitations.filter((_, i) => i !== index); }}> Accept </button>
                <button on:click={() => invitations = invitations.filter((_, i) => i !== index)}> Decline </button>
             </div>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
</div>

<style>
  .lobby { padding: 1rem; text-align: center; color: #fff; display: flex; flex-direction: column; gap: 1.5rem; }
  .online-users { margin: 0; padding: 1rem; background-color: #3a3a3a; border-radius: 8px; color: #e0e0e0; }
  .online-users h2 { margin-top: 0; color: orange; font-weight: bold; }
  .online-users ul { list-style: none; padding: 0; margin: 0; max-height: 200px; overflow-y: auto; }
  .online-users li { margin: 0.6rem 0; display: flex; justify-content: space-between; align-items: center; padding: 0.4rem; border-bottom: 1px solid #555; }
  .online-users li:last-child { border-bottom: none; }
  .error { color: #ff8080; font-size: 0.9em; }
  .status { font-size: 0.85em; margin-left: 0.5em; color: #aaa; }
  .status.available { color: lightgreen; }
  .status.error { color: #ff8080; }
  button { font-size: 1rem; padding: 0.4rem 0.8rem; border: none; background-color: #646cff; color: white; border-radius: 6px; cursor: pointer; transition: background-color 0.25s; }
  button:hover { background-color: #535bf2; }
  button:disabled, button.disabled { background-color: #555; cursor: not-allowed; opacity: 0.6; } /* Ensure disabled style applies */
  .play-button button { font-size: 1.5rem; padding: 0.8rem 1.5rem; }
  .invitations { margin: 1rem; padding: 1rem; background-color: #2a2a2a; border: 1px solid #646cff; border-radius: 8px; color: #fff; }
  .invitations h2 { margin-top: 0; color: orange; }
  .invitations ul { list-style: none; padding: 0; margin: 0; }
  .invitations li { margin: 0.7rem 0; border-bottom: 1px solid #444; padding-bottom: 0.7rem; display: flex; justify-content: space-between; align-items: center; }
  .invitations li p { margin: 0; text-align: left; }
  .invitations li div { display: flex; gap: 0.5rem; }
</style>