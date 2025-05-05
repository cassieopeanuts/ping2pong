<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher, getContext } from "svelte";
  // Import types from holochain/client
  import type { AppClient, Record, HolochainError, AgentPubKey, ActionHash, Entry } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  // Import local context and types
  import { clientContext, type ClientContext } from "../../contexts";
  import type { PlayerStatus, Game } from "../ping_2_pong/types"; // Removed GameInvitationSignal
  import { decode } from "@msgpack/msgpack";

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // --- Component State ---
  let loading: boolean = false; // For Join/Create button
  let statusMessage: string | null = null;  // Status/Error message display
  interface OnlineUser { pubKey: AgentPubKey; status: PlayerStatus | 'Loading' | 'Error'; }
  let onlineUsers: OnlineUser[] = [];
  let fetchingUsers: boolean = false; // To prevent concurrent fetches
  let fetchError: string | null = null; // Error fetching users/status
  let invitationStatus: string | null = null; // Status/Error message for sending invites

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
  loading = true;
  statusMessage = null;
  invitationStatus = null;

  if (!client) {
    statusMessage = "Holochain client not ready.";
    loading = false;
    return;
  }

  try {
    // 1. fetch every game ever created
    const allGames: Record[] = await client.callZome({
      cap_secret: null,
      role_name: "ping_2_pong",
      zome_name: "ping_2_pong",
      fn_name: "get_all_games",
      payload: null
    });

    console.log(`[Lobby] Found ${allGames.length} total games.`);

    let joinableGame: Record | null = null;
    let myWaitingGame: Record | null = null;

    // 2. walk through them, but **look at the latest revision**
    for (const original of allGames) {
      const latest: Record = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_latest_game",
        payload: original.signed_action.hashed.hash           // original hash
      });

      const decoded = decode((latest.entry as any).Present.entry) as Game;

      const waitingAndOpen =
        decoded.game_status === "Waiting" && decoded.player_2 === null;

      const isMine =
        encodeHashToBase64(decoded.player_1) ===
        encodeHashToBase64(client.myPubKey);

      if (waitingAndOpen && !isMine) {
        joinableGame = original;           // found something to join
        break;
      }

      if (waitingAndOpen && isMine) {
        myWaitingGame = original;          // remember my own “open” game
      }
    }

    // 3. act on the result of the scan
    if (joinableGame) {
      // ---- JOIN a game someone else created ----
      await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "join_game",
        payload: joinableGame.signed_action.hashed.hash
      });
      statusMessage = "Joining game… waiting for confirmation.";
    } else if (myWaitingGame) {
      // ---- I already created a waiting game earlier ----
      statusMessage = "Already waiting for an opponent in your game.";
    } else {
      // ---- CREATE a brand-new game ----
      const record: Record = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "create_game",
        payload: { player_1: client.myPubKey, player_2: null }
      });
      console.log(
        "[Lobby] Created new game, waiting:",
        encodeHashToBase64(record.signed_action.hashed.hash)
      );
      statusMessage = "Game created. Waiting for an opponent…";
    }
  } catch (e) {
    console.error("Error in joinOrCreateGame:", e);
    statusMessage = (e as HolochainError).message;
  } finally {
    loading = false;
  }
}
// Executed when "Invite" button is clicked
async function sendInvitation(invitee: AgentPubKey) {
  invitationStatus = null;
  statusMessage    = null;               // Clear UI messages first

  if (!client) {
    invitationStatus = "Holochain client not ready.";
    return;
  }

  try {
    // ── 1. Create the Game entry (still “Waiting”) ──────────────────────────
    console.log("Creating game for invitation to:", encodeHashToBase64(invitee));

    const createPayload = {                    // matches create_game input
      player_1: client.myPubKey,
      player_2: null                        // fine to pre-fill – backend will ignore if you prefer
    };

    const gameRecord: Record = await client.callZome({
      cap_secret : null,
      role_name  : "ping_2_pong",
      zome_name  : "ping_2_pong",
      fn_name    : "create_game",
      payload    : createPayload
    });

    const gameHash: ActionHash = gameRecord.signed_action.hashed.hash;
    console.log("Game created for invitation:", encodeHashToBase64(gameHash));

    // ── 2. Build *new* InvitationPayload (invitee not inviter) ─────────────
    const invitationPayload = {
      game_id : gameHash,
      invitee : invitee,                       
      message : "You have been invited to play Pong!"
    };

    // ── 3. Send the invitation via the new zome extern ─────────────────────
    console.log("Sending invitation...");
    await client.callZome({
      cap_secret : null,
      role_name  : "ping_2_pong",
      zome_name  : "ping_2_pong",
      fn_name    : "send_invitation",          // the extern you just added
      payload    : invitationPayload
    });
    console.log("Invitation sent.");

    // ── 4. Stay on the lobby; wait for GameStarted signal ──────────────────
    invitationStatus = "Invitation sent. Waiting for response...";

  } catch (e) {
    console.error("Error sending invitation:", e);
    const errData = (e as any)?.data?.data;
    invitationStatus = errData
      ? `${(e as Error).message}: ${errData}`
      : (e as Error).message;
  }
}

  // Periodically fetch online users and their game status
  async function fetchOnlineUsersAndStatus() {
    if (fetchingUsers || !client) return;
    fetchingUsers = true; fetchError = null;
    try {
      const fetchedPubKeys: AgentPubKey[] = await client.callZome({
          cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
          fn_name: "get_online_users", payload: null
        });

      const users = fetchedPubKeys.map(pubKey => ({ pubKey, status: 'Loading' } as OnlineUser));
      onlineUsers = users; // Initial set with loading status

      // Fetch status for each user individually
      for (let i = 0; i < onlineUsers.length; i++) {
            const user = onlineUsers[i];
            try {
                  const statusResult = await client.callZome({
                      cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
                      fn_name: "get_player_status", payload: user.pubKey
                  });
                  // Assign the result (should be 'Available' or 'InGame')
                  if (typeof statusResult === 'string') {
                      onlineUsers[i] = { ...user, status: statusResult as PlayerStatus };
                  } else {
                       console.warn("Unexpected status result format:", statusResult);
                       onlineUsers[i] = { ...user, status: 'Error' };
                  }
            } catch (statusError) { console.error(`Error fetching status for ${truncatePubkey(user.pubKey)}:`, statusError); onlineUsers[i] = { ...user, status: 'Error' }; }
      }
      onlineUsers = [...onlineUsers]; // Trigger Svelte reactivity

    } catch (e) {
        const errorMsg = (e as HolochainError).message;
        console.error("Error fetching online users:", errorMsg);
        if (errorMsg.includes("source chain head has moved")) {
            console.warn("Skipping online users update due to source chain conflict.");
        } else {
            fetchError = errorMsg;
            onlineUsers = []; // Clear list on other errors
        }
    } finally {
        fetchingUsers = false;
    }
  }

  // --- Lifecycle ---
  let onlineInterval: ReturnType<typeof setInterval>;

  onMount(async () => {
    client = await appClientContext.getClient();
    await fetchOnlineUsersAndStatus(); // Initial fetch
    onlineInterval = setInterval(fetchOnlineUsersAndStatus, 11000); // Fetch status periodically
  });

  onDestroy(() => {
    clearInterval(onlineInterval); // Clear interval on component destroy
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
    {#if invitationStatus} <p class:error={!invitationStatus.startsWith("Invitation sent")} style="margin-top: 10px;">{invitationStatus}</p> {/if}
  </section>

  <section class="play-button">
    {#if loading} <p>Joining/Creating Game...</p>
    {:else if statusMessage} <p class:error={!statusMessage.startsWith("Game created") && !statusMessage.startsWith("Joining game")} style="margin-top: 10px;">{statusMessage}</p>
    {:else} <button on:click={joinOrCreateGame}>Play Random</button> {/if}
  </section>

</div>

<style>
  /* ... (keep existing styles) ... */
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
  button:disabled, button.disabled { background-color: #555; cursor: not-allowed; opacity: 0.6; }
  .play-button button { font-size: 1.5rem; padding: 0.8rem 1.5rem; }
  /* Styles for status/error messages */
  p.error { color: #ff8080; } /* Red for errors */
  p:not(.error) { color: #ccc; } /* Grey/white for info/waiting messages */
</style>
