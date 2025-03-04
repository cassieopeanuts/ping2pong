<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { getContext } from "svelte";
  import type { AppClient, Record, HolochainError, AgentPubKey } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  import { clientContext } from "../../contexts";
  import type { ClientContext } from "../../contexts";

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // State for joining/creating game.
  let loading: boolean = false;
  let error: string | null = null;

  // State for online users.
  let onlineUsers: AgentPubKey[] = [];
  let lastOnlineUsers: AgentPubKey[] = [];
  let lastOnlineUpdateTime: number = 0;
  let fetchingUsers: boolean = false;
  let fetchError: string | null = null;

  // For invitations.
  let invitationError: string | null = null;
  // Invitations we receive (we assume the invitation signal payload includes the inviter, game_id and a message)
  let invitations: { game_id: string; inviter: AgentPubKey; message: string }[] = [];

  // Helper: Convert and truncate an AgentPubKey.
  function truncatePubkey(pubkey: AgentPubKey): string {
    const base64 = encodeHashToBase64(pubkey);
    return base64.slice(0, 8) + "..." + base64.slice(-6);
  }

  // Function to join or create a random game.
  async function joinOrCreateGame() {
    loading = true;
    error = null;
    try {
      const result = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_all_games",
        payload: null,
      });
      let waitingGame: Record | null = null;
      if (result && Array.isArray(result)) {
        const waitingGames = result.filter((r: Record) => {
          const entry = (r.entry as any)?.Present?.entry;
          if (!entry || !entry.game_status) {
            console.log("Skipping record without valid game entry:", r);
            return false;
          }
          return entry.game_status.type === "Waiting";
        });
        if (waitingGames.length > 0) {
          waitingGame = waitingGames[0];
        }
      }
      if (waitingGame) {
        dispatch("join-game", { gameHash: waitingGame.signed_action.hashed.hash });
      } else {
        const newGameRecord: Record = await client.callZome({
          cap_secret: null,
          role_name: "ping_2_pong",
          zome_name: "ping_2_pong",
          fn_name: "create_game",
          payload: {
            player_1: client.myPubKey,
            player_2: null,
            created_at: Date.now(),
            game_status: { type: "Waiting" },
            player_1_paddle: 250,
            player_2_paddle: 250,
            ball_x: 400,
            ball_y: 300,
          },
        });
        dispatch("join-game", { gameHash: newGameRecord.signed_action.hashed.hash });
      }
    } catch (e) {
      error = (e as HolochainError).message;
    } finally {
      loading = false;
    }
  }

  // Function to send an invitation to a specific user.
  async function sendInvitation(invitee: AgentPubKey) {
    try {
      // Create a new game entry with the invited user set as player2.
      const gameEntry = {
        player_1: client.myPubKey,
        player_2: invitee,
        created_at: Date.now(),
        // We set the status to "Waiting" until the invitee accepts.
        game_status: { type: "Waiting" },
        player_1_paddle: 250,
        player_2_paddle: 250,
        ball_x: 400,
        ball_y: 300,
      };
      const gameRecord: Record = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "create_game",
        payload: gameEntry,
      });
      // Now, send a GameInvitation signal.
      const invitationSignal = {
        type: "GameInvitation",
        game_id: gameRecord.signed_action.hashed.hash,
        inviter: client.myPubKey,
        message: "You have been invited to play Pong!",
      };
      await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "send_signal",
        payload: invitationSignal,
      });
      // Optionally, join the game immediately for the inviter.
      dispatch("join-game", { gameHash: gameRecord.signed_action.hashed.hash });
    } catch (e) {
      console.error("Error sending invitation:", e);
      invitationError = (e as HolochainError).message;
    }
  }

  // Publish presence to signal the agent is online.
  async function publishPresence() {
    try {
      const result = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "publish_presence",
        payload: null,
      });
      console.log("Presence published:", result);
    } catch (e) {
      console.error("Error publishing presence:", e);
    }
  }

  // Fetch online users from the DNA.
  async function fetchOnlineUsers() {
    fetchingUsers = true;
    fetchError = null;
    try {
      const result = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_online_users",
        payload: null,
      });
      let fetchedUsers: AgentPubKey[] = [];
      if (result && Array.isArray(result)) {
        fetchedUsers = result;
      }
      // Use cached online users if new fetch returns empty and a recent non-empty result exists.
      const now = Date.now();
      if (
        fetchedUsers.length === 0 &&
        (now - lastOnlineUpdateTime) < 20000 &&
        lastOnlineUsers.length > 0
      ) {
        console.log("Using cached online users");
        onlineUsers = lastOnlineUsers;
      } else {
        onlineUsers = fetchedUsers;
        if (fetchedUsers.length > 0) {
          lastOnlineUsers = fetchedUsers;
          lastOnlineUpdateTime = now;
        }
      }
    } catch (e) {
      fetchError = (e as HolochainError).message;
    } finally {
      fetchingUsers = false;
    }
  }

  // --- Signal handling for invitations ---
  // Listen for invitation signals (ephemeral).
  function subscribeToInvitations() {
    return client.on("signal", (signal: any) => {
      if (signal.type === "GameInvitation") {
        // If the invitation is not from the current agent.
        if (signal.inviter.toString() !== client.myPubKey.toString()) {
          console.log("Received invitation signal:", signal);
          // Add the invitation to the local state.
          invitations = [...invitations, {
            game_id: signal.game_id,
            inviter: signal.inviter,
            message: signal.message
          }];
        }
      }
    });
  }

  let presenceInterval: ReturnType<typeof setInterval>;
  let onlineInterval: ReturnType<typeof setInterval>;
  let invitationUnsub: ReturnType<typeof client.on>;

  onMount(async () => {
    client = await appClientContext.getClient();
    // Initial publish and fetch.
    await publishPresence();
    await fetchOnlineUsers();
    // Set intervals.
    presenceInterval = setInterval(publishPresence, 15000); // every 15 sec
    onlineInterval = setInterval(fetchOnlineUsers, 10000);   // every 10 sec
    // Subscribe to invitation signals.
    invitationUnsub = subscribeToInvitations();
  });

  onDestroy(() => {
    clearInterval(presenceInterval);
    clearInterval(onlineInterval);
    if (invitationUnsub) invitationUnsub();
  });
</script>

<div class="lobby">
  <section class="online-users">
    <h2>Online Users</h2>
    {#if fetchingUsers}
      <p>Loading online users...</p>
    {:else if fetchError}
      <p class="error">Error: {fetchError}</p>
    {:else if onlineUsers.length === 0}
      <p>No users online</p>
    {:else}
      <ul>
        {#each onlineUsers as user}
          {#if user.toString() !== client.myPubKey.toString()}
            <li>
              {truncatePubkey(user)}
              <button on:click={() => sendInvitation(user)}>Invite</button>
            </li>
          {/if}
        {/each}
      </ul>
    {/if}
  </section>

  <section class="play-button">
    {#if loading}
      <p>Loading...</p>
    {:else if error}
      <p class="error">Error: {error}</p>
    {:else}
      <button on:click={joinOrCreateGame}>Play Random</button>
    {/if}
  </section>

  <!-- Invitation Popup (for demonstration, a simple list of invitations) -->
  {#if invitations.length > 0}
    <section class="invitations">
      <h2>Game Invitations</h2>
      <ul>
        {#each invitations as inv, index (inv.game_id)}
          <li>
            <p>
              <strong>Invitation from:</strong> {truncatePubkey(inv.inviter)}
              <br>
              <em>{inv.message}</em>
            </p>
            <button on:click={() => dispatch("join-game", { gameHash: inv.game_id })}>
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
    gap: 2rem;
  }
  .online-users {
    margin: 1rem 0;
    padding: 1rem;
    background-color: #333;
    border-radius: 8px;
    color: orange;
    font-weight: bold;
  }
  .online-users ul {
    list-style: none;
    padding: 0;
  }
  .online-users li {
    margin: 0.5rem 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .error {
    color: #ff8080;
  }
  button {
    font-size: 1.5rem;
    padding: 0.5rem 1rem;
    border: none;
    background-color: #646cff;
    color: white;
    border-radius: 8px;
    cursor: pointer;
    transition: background-color 0.25s;
  }
  button:hover {
    background-color: #535bf2;
  }
  .invitations {
    margin: 1rem;
    padding: 1rem;
    background-color: #222;
    border: 2px solid #646cff;
    border-radius: 8px;
    color: #fff;
  }
  .invitations ul {
    list-style: none;
    padding: 0;
  }
  .invitations li {
    margin: 0.5rem 0;
    border-bottom: 1px solid #646cff;
    padding: 0.5rem;
  }
</style>
