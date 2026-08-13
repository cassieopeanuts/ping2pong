<script lang="ts">
  import { onMount, onDestroy, getContext } from "svelte";
  import type { AppClient, AgentPubKey, AgentPubKeyB64 } from "@holochain/client"; // Added AgentPubKeyB64
  import { encodeHashToBase64 } from "@holochain/client"; // For converting raw AgentPubKey
  import { clientContext, type ClientContext } from "../../contexts";
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "../../holochainConfig";
  import { get as getStoreValue } from "svelte/store";
  import { getOrFetchProfile, profilesCache, type DisplayProfile } from "../../stores/profilesStore";
  import { truncatePubkey } from "../../utils";

  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);
  let isMounted = true;
  
  interface LeaderboardEntryData {
      player_key_b64: AgentPubKeyB64; // Storing as B64 string for map keys and direct use
      nickname?: string;
      total_points: number;
      games_played: number;
  }
  let leaderboardData: LeaderboardEntryData[] = [];
  let isLoading: boolean = true;
  let errorMessage: string | null = null;

  function getNickname(playerKeyB64: AgentPubKeyB64, fallbackNickname?: string, _cacheMap?: Map<AgentPubKeyB64, DisplayProfile>): string {
      if (fallbackNickname) return fallbackNickname;
      const cached = $profilesCache.get(playerKeyB64);
      if (cached && cached.nickname) return cached.nickname;
      return truncatePubkey(playerKeyB64, 6, 4);
  }

  onMount(async () => {
    try {
      const fetchedClient = await appClientContext.getClient();
      if (!isMounted) return;
      client = fetchedClient;
      await fetchLeaderboard();
    } catch (e: any) {
      if (!isMounted) return;
      console.error("Error initializing leaderboard:", e);
      errorMessage = e.message || "Failed to initialize leaderboard client.";
      isLoading = false;
    }
  });

  onDestroy(() => {
    isMounted = false;
  });

  async function fetchLeaderboard() {
    isLoading = true;
    errorMessage = null;
    if (!isMounted || !client) {
      if (!isMounted) return;
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

      if (!isMounted) return;

      if (!rawLeaderboardEntries) {
        leaderboardData = [];
        isLoading = false;
        return;
      }
      
      const processedEntries = rawLeaderboardEntries.map(rawEntry => {
        const b64 = encodeHashToBase64(rawEntry.player_key);
        const cached = $profilesCache.get(b64);
        return {
          player_key_b64: b64,
          nickname: cached?.nickname,
          total_points: rawEntry.total_points,
          games_played: rawEntry.games_played,
        };
      });
      leaderboardData = processedEntries;

      // Asynchronously fetch nicknames for each entry
      await Promise.all(processedEntries.map(async (entryData, index) => {
        const profile = await getOrFetchProfile(client, entryData.player_key_b64, true);
        if (profile && profile.nickname) {
          leaderboardData[index] = { ...leaderboardData[index], nickname: profile.nickname };
        }
      }));
      leaderboardData = [...leaderboardData];

    } catch (e: any) {
      console.error("Error fetching leaderboard data:", e);
      errorMessage = e.data?.data || e.message || "Failed to fetch leaderboard.";
      leaderboardData = [];
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
          <th style="width: 15%;">#</th>
          <th style="width: 45%;">Player</th>
          <th style="width: 20%;">Pts</th>
          <th style="width: 20%;">Played</th>
        </tr>
      </thead>
      <tbody>
        {#each leaderboardData as entry, i}
          <tr>
            <td>{i + 1}</td>
            <td title={entry.player_key_b64}>{getNickname(entry.player_key_b64, entry.nickname, $profilesCache)}</td>
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
    padding: 0.75rem;
    background: var(--container-bg-color);
    color: var(--secondary-text-color);
    text-align: center;
    border-radius: 0px;
    border: 2px solid var(--border-color);
    width: 100%;
    box-sizing: border-box;
    overflow-x: hidden;
  }
  .leaderboard h3 {
    color: var(--primary-text-color);
    margin-bottom: 0.5rem;
    font-size: 1.1rem;
    line-height: 1.2;
  }
  table {
    width: 100%;
    table-layout: fixed;
    border-collapse: collapse; 
    margin-top: 0.5rem;
    font-size: 0.7rem;
  }
  th, td {
    border: 2px solid var(--border-color); 
    padding: 0.4em 0.2em;
    text-align: center;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  th {
    background-color: var(--secondary-bg-color); 
    color: var(--primary-text-color); 
  }
  td {
    color: var(--secondary-text-color);
  }
  /* Loading/error messages will use global styles from index.css */
  /* Ensure fallback paragraph text is also themed if needed */
  .leaderboard > p:not(.loading-message):not(.error-message) {
    color: var(--text-muted-color);
    /* font-size is 1em (16px) from global <p> style, which is fine */
  }

  /* Override for .loading-message specifically within leaderboard context if needed */
  .leaderboard :global(.loading-message) {
    /* Using :global as .loading-message is defined in index.css */
    /* Alternatively, just define .loading-message here if it should be unique to leaderboard */
    font-size: 1rem; /* 16px. Global .loading-message is 1.2em (19.2px) */
  }
  /* .error-message already uses 1em (16px) globally, which is fine */
</style>
