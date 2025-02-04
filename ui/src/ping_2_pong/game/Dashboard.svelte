<script lang="ts">
  import Leaderboard from "./Leaderboard.svelte";
  import Lobby from "./Lobby.svelte";
  import PlayButton from "./PlayButton.svelte";
  import { currentRoute } from "../../stores/routeStore";
  
  function handlePlay() {
    currentRoute.set("gameplay");
  }
</script>

<div class="dashboard">
  <div class="sidebar left">
    <Leaderboard />
  </div>
  <div class="center">
    <PlayButton on:play={handlePlay} />
  </div>
  <div class="sidebar right">
    <Lobby />
  </div>
</div>

<style>
  .dashboard {
    display: grid;
    grid-template-columns: 1fr 2fr 1fr;
    height: 100vh;
    background: #222;
  }
  .sidebar {
    background: #333;
    color: #fff;
    padding: 1rem;
    overflow-y: auto;
  }
  .center {
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
