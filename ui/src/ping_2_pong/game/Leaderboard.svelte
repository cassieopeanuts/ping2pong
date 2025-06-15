<script lang="ts">
  import { onMount, getContext } from "svelte";
  import type { AppClient, AgentPubKey, AgentPubKeyB64 } from "@holochain/client"; 
  import { encodeHashToBase64 } from "@holochain/client"; 
  import { clientContext, type ClientContext } from "../../contexts";
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "../../holochainConfig";
  import { getOrFetchProfile, type DisplayProfile } from "../../stores/profilesStore";
  import { truncatePubkey } from "../../utils";

  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);
  
  interface LeaderboardEntryData {
      player_key_b64: AgentPubKeyB64; // Storing as B64 string for map keys and direct use
      nickname?: string;
      total_points: number;
      games_played: number;
  }
  let leaderboardData: LeaderboardEntryData[] = [];
  let isLoading: boolean = true;
  let errorMessage: string | null = null;

  onMount(async () => {
    try {
      client = await appClientContext.getClient();
      await fetchLeaderboard();
    } catch (e: any) {
      console.error("Error initializing leaderboard:", e);
      errorMessage = e.message || "Failed to initialize leaderboard client.";
      isLoading = false;
    }
  });

  async function fetchLeaderboard() {
    isLoading = true;
    errorMessage = null;
    if (!client) {
      errorMessage = "Client not initialized.";
      isLoading = false;
      return;
    }

    try {
      const rawLeaderboardEntries: Array<{player_key: AgentPubKey, total_points: number, games_played: number}> = 
        await client.callZome({
            cap_secret: null,
            role_name: HOLOCHAIN_ROLE_NAME,
            zome_name: HOLOCHAIN_ZOME_NAME,
            fn_name: "get_leaderboard_data",
            payload: null,
      });

      if (!rawLeaderboardEntries) {
        leaderboardData = [];
        isLoading = false;
        return;
      }
      
      const processedEntries = rawLeaderboardEntries.map(rawEntry => ({
          player_key_b64: encodeHashToBase64(rawEntry.player_key),
          nickname: undefined, // Placeholder, to be filled
          total_points: rawEntry.total_points,
          games_played: rawEntry.games_played,
      }));
      leaderboardData = processedEntries;

      // Asynchronously fetch nicknames for each entry
      // Use Promise.all to wait for all nickname fetches if desired, or update reactively
      await Promise.all(processedEntries.map(async (entryData, index) => {
        const profile = await getOrFetchProfile(client, entryData.player_key_b64); // Pass B64 key
        if (profile && profile.nickname) {
          // Create a new object for the specific entry to ensure reactivity if needed,
          // or reassign the whole array as done below.
          leaderboardData[index] = { ...leaderboardData[index], nickname: profile.nickname };
        }
      }));
      leaderboardData = [...leaderboardData]; // Trigger Svelte reactivity after all potential updates

    } catch (e: any) {
      console.error("Error fetching leaderboard data:", e);
      errorMessage = e.data?.data || e.message || "Failed to fetch leaderboard.";
      leaderboardData = []; // Clear data on error
    } finally {
      isLoading = false;
    }
  }
</script>

<div class="leaderboard">
  <h3>Leaderboard</h3>
  {#if isLoading}
    <p class="loading-message">Loading Leaderboard...</p>
  {:else if errorMessage}
    <p class="error-message">{errorMessage}</p>
  {:else if leaderboardData.length === 0}
    <p>No leaderboard data yet. Play some games!</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Rank</th>
          <th>Player</th>
          <th>Total Points</th>
          <th>Games Played</th>
        </tr>
      </thead>
      <tbody>
        {#each leaderboardData as entry, i}
          <tr>
            <td>{i + 1}</td>
            <td title={entry.player_key_b64}>{entry.nickname || truncatePubkey(entry.player_key_b64, 6, 4)}</td>
            <td>{entry.total_points}</td>
            <td>{entry.games_played}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .leaderboard {
    padding: 1rem;
    background: var(--container-bg-color);
    color: var(--secondary-text-color);
    text-align: center;
    border-radius: 0px; 
    border: 2px solid var(--border-color); 
    width: 100%; 
    box-sizing: border-box;
  }
  .leaderboard h3 {
    color: var(--primary-text-color); 
    margin-bottom: 1rem;
    font-size: 1.25rem;
    line-height: 1.2;  
  }
  table {
    width: 100%;
    border-collapse: collapse; 
    margin-top: 1rem;
    font-size: 0.75rem; 
  }
  th, td {
    border: 2px solid var(--border-color); 
    padding: 0.5em;
    text-align: left;
  }
  th {
    background-color: var(--secondary-bg-color); 
    color: var(--primary-text-color); 
  }
  td {
    color: var(--secondary-text-color);
  }
  .leaderboard > p:not(.loading-message):not(.error-message) {
    color: var(--text-muted-color);
  }

  .leaderboard :global(.loading-message) {
    font-size: 1rem; 
  }
</style>
