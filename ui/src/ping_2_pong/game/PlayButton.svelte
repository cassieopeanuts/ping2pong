<script lang="ts">
  import { createEventDispatcher } from "svelte";
  const dispatch = createEventDispatcher();

  function play() {
    dispatch("play");
  }
</script>

<button on:click={play} class="play-button">
  Play
</button>

<style>
  .play-button {
    font-size: 2rem;
    padding: 1rem 2rem;
    border: none;
    background-color: #646cff;
    color: white;
    border-radius: 8px;
    cursor: pointer;
    transition: background-color 0.25s;
  }
  .play-button:hover {
    background-color: #535bf2;
  }
</style>
