<script lang="ts">
  import Leaderboard from "./Leaderboard.svelte";
  import type { Leaderboard as LeaderboardType } from "./Leaderboard.svelte"; 
  import Lobby from "./Lobby.svelte";
  import PlayButton from "./PlayButton.svelte";
  import GlobalChat from "../chat/GlobalChat.svelte"; 
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher();

  let leaderboardComponent: LeaderboardType; 

  export function refreshLeaderboardData() {
    if (leaderboardComponent && typeof leaderboardComponent.fetchLeaderboard === 'function') {
      console.log("[Dashboard.svelte] Calling fetchLeaderboard on Leaderboard component.");
      leaderboardComponent.fetchLeaderboard();
    } else {
      console.warn("[Dashboard.svelte] Leaderboard component or fetchLeaderboard method not available for refresh.");
    }
  }
</script>

<div class="dashboard-layout">
  <div class="dashboard-col-left">
    <Leaderboard bind:this={leaderboardComponent} />
  </div>
  <div class="dashboard-col-center">
    <PlayButton />
    <GlobalChat />
  </div>
  <div class="dashboard-col-right">
    <!-- Lobby dispatches join-game events -->
    <Lobby on:join-game={(e) => dispatch("join-game", e.detail)} />
  </div>
</div>
