<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher, getContext } from "svelte";
  // Import types from holochain/client
  import type { AppClient, Record, HolochainError, AgentPubKey, ActionHash, Entry } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  // Import local context and types
  import { clientContext, type ClientContext } from "../../contexts";
  import type { PlayerStatus, Game } from "../ping_2_pong/types"; // Removed GameInvitationSignal
  import { decode } from "@msgpack/msgpack";
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "../../holochainConfig";
  import { getOrFetchProfile, type DisplayProfile } from "../../stores/profilesStore"; // Import profile store
  import { truncatePubkey } from "../../utils"; // Import global truncatePubkey

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // --- Component State ---
  interface OnlineUser {
    pubKey: AgentPubKey;
    status: PlayerStatus | 'Loading' | 'Error';
    nickname?: string;
    pubKeyB64: string; // Store B64 for easier keying and display fallback
  }
  let onlineUsers: OnlineUser[] = [];
  let fetchingUsers: boolean = false; // To prevent concurrent fetches
  let fetchError: string | null = null; // Error fetching users/status
  let invitationStatus: string | null = null; // Status/Error message for sending invites

  // --- Helper Functions ---
  // Local truncatePubkey REMOVED - using imported one

  // --- Zome Calls & Logic ---

  // Executed when "Play Random" is clicked - MOVED to PlayButton.svelte
  // async function joinOrCreateGame() { ... }

  // Executed when "Invite" button is clicked
  async function sendInvitation(invitee: AgentPubKey) {
  invitationStatus = null;
  // statusMessage    = null; // No longer used here for joinOrCreateGame status

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
      role_name  : HOLOCHAIN_ROLE_NAME,
      zome_name  : HOLOCHAIN_ZOME_NAME,
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
      role_name  : HOLOCHAIN_ROLE_NAME,
      zome_name  : HOLOCHAIN_ZOME_NAME,
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
    fetchingUsers = true;
    fetchError = null;
    try {
      const fetchedPubKeys: AgentPubKey[] = await client.callZome({
          cap_secret: null, role_name: HOLOCHAIN_ROLE_NAME, zome_name: HOLOCHAIN_ZOME_NAME,
          fn_name: "get_online_users", payload: null
        });

      // Create initial user list with pubKey and loading status for nickname/status
      const newOnlineUsers: OnlineUser[] = fetchedPubKeys.map(pubKey => ({
        pubKey,
        status: 'Loading',
        pubKeyB64: encodeHashToBase64(pubKey) // Store B64 version
      }));
      onlineUsers = newOnlineUsers;

      // Fetch profiles and statuses for each user
      for (let i = 0; i < onlineUsers.length; i++) {
        const user = onlineUsers[i];

        // Fetch profile (nickname)
        getOrFetchProfile(client, user.pubKey).then(profile => {
          if (profile) {
            onlineUsers[i] = { ...onlineUsers[i], nickname: profile.nickname };
            onlineUsers = [...onlineUsers]; // Trigger reactivity
          }
        });

        // Fetch status
        client.callZome({
            cap_secret: null, role_name: HOLOCHAIN_ROLE_NAME, zome_name: HOLOCHAIN_ZOME_NAME,
            fn_name: "get_player_status", payload: user.pubKey
        }).then(statusResult => {
            if (typeof statusResult === 'string') {
                onlineUsers[i] = { ...onlineUsers[i], status: statusResult as PlayerStatus };
            } else {
                 console.warn("Unexpected status result format:", statusResult);
                 onlineUsers[i] = { ...onlineUsers[i], status: 'Error' };
            }
            onlineUsers = [...onlineUsers]; // Trigger reactivity
        }).catch(statusError => {
            console.error(`Error fetching status for ${truncatePubkey(user.pubKeyB64)}:`, statusError); // Use B64 for logging
            onlineUsers[i] = { ...onlineUsers[i], status: 'Error' };
            onlineUsers = [...onlineUsers]; // Trigger reactivity
        });
      }
      // Initial render might show loading, then updates as promises resolve
      // No need for final onlineUsers = [...onlineUsers] here as it's done within loops

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
    {#if fetchingUsers && onlineUsers.length === 0} <p class="loading-message">Loading online users...</p> <!-- Use global class -->
    {:else if fetchError} <p class="error-message">Error fetching users: {fetchError}</p> <!-- Use global class -->
    {:else if onlineUsers.filter(u => u.pubKeyB64 !== encodeHashToBase64(client?.myPubKey)).length === 0}
      <p>No other users online</p>
    {:else}
      <ul>
        {#each onlineUsers as user (user.pubKeyB64)}
          {#if user.pubKeyB64 !== encodeHashToBase64(client?.myPubKey)}
            {@const isDisabled = !(user.status === 'Available')}
            <li>
              <span class="user-details" title={user.pubKeyB64}>
                <span class="nickname">{user.nickname || truncatePubkey(user.pubKeyB64, 6, 4)}</span>
                <span class="dots"></span>
                {#if user.status === 'Loading'}
                  <span class="status status-loading">LOADING</span>
                {:else if user.status === 'Error'}
                  <span class="status status-error">ERROR</span>
                {:else if user.status === 'InGame'}
                  <span class="status status-ingame">PLAYING</span>
                {:else if user.status === 'Available'}
                  <span class="status status-available">READY</span>
                {:else}
                  <span class="status status-unknown">OFFLINE</span>
                {/if}
              </span>
              <button on:click={() => sendInvitation(user.pubKey)} disabled={isDisabled} class:disabled={isDisabled}> Invite </button>
            </li>
          {/if}
        {/each}
      </ul>
    {/if}
    {#if invitationStatus} <p class:error={!invitationStatus.startsWith("Invitation sent")} style="margin-top: 10px;">{invitationStatus}</p> {/if}
  </section>

  <!-- Play Random Button Section REMOVED -->
  <!-- <section class="play-button"> ... </section> -->

</div>

<style>
  .lobby {
    padding: 1rem;
    text-align: center;
    color: var(--secondary-text-color); /* Was #fff */
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  .online-users {
    margin: 0;
    padding: 1rem;
    background-color: var(--container-bg-color); /* Was #3a3a3a */
    border-radius: 8px;
    color: var(--secondary-text-color); /* Was #e0e0e0 */
  }
  .online-users h2 {
    margin-top: 0;
    color: var(--primary-text-color); /* Was orange */
    font-weight: bold; /* Kept bold as it's a heading style */
    font-size: 1.25rem; /* 20px. Global h2 is 2.2em (35.2px) */
    line-height: 1.2;
  }
  .online-users ul {
    list-style: none;
    padding: 0;
    margin: 0;
    max-height: 200px; /* Kept as is, functional style */
    overflow-y: auto; /* Kept as is, functional style */
  }
  .online-users li {
    font-size: 0.8rem;
    line-height: 1.4;
    margin: 0.8rem 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem;
    border: 2px solid var(--border-color);
    background-color: var(--primary-bg-color);
  }
  .user-details {
    display: flex;
    align-items: center;
    flex-grow: 1;
    margin-right: 1.5rem;
  }
  .nickname {
    color: var(--secondary-text-color);
  }
  .dots {
    flex-grow: 1;
    border-bottom: 2px dotted var(--primary-text-color);
    margin: 0 12px;
    height: 10px;
  }
  .status {
    font-size: 0.7rem;
    padding: 2px 6px;
    border: 1px solid currentColor;
    font-weight: bold;
    display: inline-block;
  }
  .status-loading {
    color: #888888;
    border-color: #888888;
  }
  .status-error {
    color: #ff3333;
    border-color: #ff3333;
  }
  .status-ingame {
    color: #ff9900;
    border-color: #ff9900;
  }
  .status-available {
    color: #55ff55;
    border-color: #55ff55;
  }
  .status-unknown {
    color: #888888;
    border-color: #888888;
  }
  .error {
    color: var(--error-text-color);
  }

  /* Button styles within Lobby - these are specific and override global button styles if needed, or complement them */
  .online-users button { /* Targeting invite buttons */
    font-size: 0.75rem; /* 12px. Made smaller for list context */
    padding: 0.4rem 0.8rem; /* Padding kept, will scale with font if em based, but here it's fixed rem */
    /* border: 1px solid transparent; */ /* Old border style */
    border-width: 2px;
    border-style: solid;
    border-color: var(--primary-text-color);
    background-color: var(--button-bg-color);
    color: var(--button-text-color);
    /* border-radius: 6px; */ /* Removed for blocky style */
    cursor: pointer;
    transition: background-color 0.25s, border-color 0.25s; /* Consistent with global */
  }
  .online-users button:hover {
    background-color: var(--button-hover-bg-color);
    border-color: var(--primary-text-color); /* Consistent with global */
  }
  .online-users button:disabled, .online-users button.disabled {
    background-color: var(--disabled-bg-color);
    color: var(--disabled-text-color);
    border-color: var(--disabled-bg-color); /* Ensure border matches disabled bg */
    cursor: not-allowed;
    opacity: 1; /* Global button styles might have opacity, explicitly set to 1 to rely on text/bg colors */
  }

  /* Play Random button in .play-button section - this seems to be styled by global button styles already via PlayButton.svelte */
  /* .play-button button { font-size: 1.5rem; padding: 0.8rem 1.5rem; } */
  /* This style in PlayButton.svelte might need to be updated or use global button and scale with em or specific class */


  /* Styles for status/error messages text (not the .error-message class block) */
  /* p.error is covered by .error above if it's just text color */
  /* For other p tags that display status */
  .lobby p { /* General paragraphs in lobby, including status and "No other users" */
    font-size: 0.875rem; /* 14px */
    line-height: 1.4;
    color: var(--text-muted-color); /* Default for these paragraphs */
  }

  /* Ensure that if a p tag has class 'error', it primarily uses the .error class for color */
  .lobby p.error {
    color: var(--error-text-color); /* Explicitly ensure error color from .error class */
    /* font-size will be 0.875rem from the .lobby p rule above */
  }

  /* Override for .loading-message specifically within lobby context */
  .lobby :global(.loading-message) {
    font-size: 1rem; /* 16px. Global .loading-message is 1.2em (19.2px) */
  }
  /* .error-message class from index.css will apply its own styles (1em font size), which is fine. */
</style>
