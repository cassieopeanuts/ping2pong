<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher, getContext } from "svelte";
  import type { AppClient, Record, HolochainError, AgentPubKey, ActionHash, Entry } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  import { clientContext, type ClientContext } from "../../contexts";
  import type { PlayerStatus, Game } from "../ping_2_pong/types"; 
  import { decode } from "@msgpack/msgpack";
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "../../holochainConfig";
  import { getOrFetchProfile, type DisplayProfile } from "../../stores/profilesStore"; 
  import { truncatePubkey } from "../../utils"; 

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // --- Component State ---
  interface OnlineUser {
    pubKey: AgentPubKey;
    status: PlayerStatus | 'Loading' | 'Error';
    nickname?: string;
    pubKeyB64: string; 
  }
  let onlineUsers: OnlineUser[] = [];
  let fetchingUsers: boolean = false; 
  let fetchError: string | null = null; 
  let invitationStatus: string | null = null; 

  // Executed when "Invite" button is clicked
  async function sendInvitation(invitee: AgentPubKey) {
  invitationStatus = null;

  if (!client) {
    invitationStatus = "Holochain client not ready.";
    return;
  }

  try {
    // ── 1. Create the Game entry (still “Waiting”) ──────────────────────────
    console.log("Creating game for invitation to:", encodeHashToBase64(invitee));

    const createPayload = {                    
      player_1: client.myPubKey,
      player_2: null                       
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

    // ── 2. Build InvitationPayload  ─────────────
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
      fn_name    : "send_invitation",          
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
        pubKeyB64: encodeHashToBase64(pubKey) 
      }));
      onlineUsers = newOnlineUsers;

      // Fetch profiles and statuses for each user
      for (let i = 0; i < onlineUsers.length; i++) {
        const user = onlineUsers[i];

        // Fetch profile (nickname)
        getOrFetchProfile(client, user.pubKey).then(profile => {
          if (profile) {
            onlineUsers[i] = { ...onlineUsers[i], nickname: profile.nickname };
            onlineUsers = [...onlineUsers]; 
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
            onlineUsers = [...onlineUsers]; 
        }).catch(statusError => {
            console.error(`Error fetching status for ${truncatePubkey(user.pubKeyB64)}:`, statusError); 
            onlineUsers[i] = { ...onlineUsers[i], status: 'Error' };
            onlineUsers = [...onlineUsers]; 
        });
      }

    } catch (e) {
        const errorMsg = (e as HolochainError).message;
        console.error("Error fetching online users:", errorMsg);
        if (errorMsg.includes("source chain head has moved")) {
            console.warn("Skipping online users update due to source chain conflict.");
        } else {
            fetchError = errorMsg;
            onlineUsers = []; 
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
    {#if fetchingUsers && onlineUsers.length === 0} <p class="loading-message">Loading online users...</p> 
    {:else if fetchError} <p class="error-message">Error fetching users: {fetchError}</p> 
    {:else if onlineUsers.filter(u => u.pubKeyB64 !== encodeHashToBase64(client?.myPubKey)).length === 0}
      <p>No other users online</p>
    {:else}
      <ul>
        {#each onlineUsers as user (user.pubKeyB64)}
          {#if user.pubKeyB64 !== encodeHashToBase64(client?.myPubKey)}
            {@const isDisabled = !(user.status === 'Available')}
            <li>
              <span title={user.pubKeyB64}>
                {user.nickname || truncatePubkey(user.pubKeyB64, 6, 4)} 
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

</div>

<style>
  .lobby {
    padding: 1rem;
    text-align: center;
    color: var(--secondary-text-color); 
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  .online-users {
    margin: 0;
    padding: 1rem;
    background-color: var(--container-bg-color); 
    border-radius: 8px;
    color: var(--secondary-text-color); 
  }
  .online-users h2 {
    margin-top: 0;
    color: var(--primary-text-color); 
    font-weight: bold; 
    font-size: 1.25rem; 
    line-height: 1.2;
  }
  .online-users ul {
    list-style: none;
    padding: 0;
    margin: 0;
    max-height: 200px; 
    overflow-y: auto; 
  }
  .online-users li {
    font-size: 0.875rem; 
    line-height: 1.4;
    margin: 0.6rem 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.4rem;
    border-bottom: 1px solid var(--border-color); 
  }
  .online-users li:last-child {
    border-bottom: none;
  }
  .error { 
    color: var(--error-text-color); 
  }
  .status {
    font-size: 0.75rem; 
    line-height: 1.2;
    margin-left: 0.5em; 
    color: var(--text-muted-color); 
  }
  .status.available {
    color: var(--success-text-color); 
  }
  .status.error { 
    color: var(--error-text-color); 
  }

  /* Button styles within Lobby - these are specific and override global button styles if needed, or complement them */
  .online-users button { 
    font-size: 0.75rem;
    padding: 0.4rem 0.8rem; 
    border-width: 2px;
    border-style: solid;
    border-color: var(--primary-text-color);
    background-color: var(--button-bg-color);
    color: var(--button-text-color);
    cursor: pointer;
    transition: background-color 0.25s, border-color 0.25s;
  }
  .online-users button:hover {
    background-color: var(--button-hover-bg-color);
    border-color: var(--primary-text-color); 
  }
  .online-users button:disabled, .online-users button.disabled {
    background-color: var(--disabled-bg-color);
    color: var(--disabled-text-color);
    border-color: var(--disabled-bg-color); 
    cursor: not-allowed;
    opacity: 1; 
  }

  .lobby p { 
    font-size: 0.875rem; 
    line-height: 1.4;
    color: var(--text-muted-color); 
  }

  .lobby p.error {
    color: var(--error-text-color); 
  }

  .lobby :global(.loading-message) {
    font-size: 1rem; 
  }
</style>
