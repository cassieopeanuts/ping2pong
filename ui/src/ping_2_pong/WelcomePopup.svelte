<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { writable } from "svelte/store";
  
    // A simple store to hold the player's profile.
    export const playerProfile = writable<{ nickname: string } | null>(null);
  
    const dispatch = createEventDispatcher();
    let nickname: string = "";
  
    function register() {
      if (nickname.trim() !== "") {
        // Update the global playerProfile store.
        playerProfile.set({ nickname });
        // Dispatch a 'registered' event so the parent knows registration is complete.
        dispatch("registered", { nickname });
      } else {
        alert("Please enter a nickname.");
      }
    }
  </script>
  
  <div class="popup">
    <h2>Welcome! Let's Pong to Ping!</h2>
    <input type="text" placeholder="Enter your nickname" bind:value={nickname} />
    <button on:click={register}>Register</button>
  </div>
  
  <style>
    .popup {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: rgba(0, 0, 0, 0.7);
      display: flex;
      flex-direction: column;
      justify-content: center;
      align-items: center;
      z-index: 1000;
      color: #fff;
    }
    input {
      margin: 1rem 0;
      padding: 0.5rem;
      font-size: 1rem;
    }
    button {
      padding: 0.5rem 1rem;
      font-size: 1rem;
      border: none;
      border-radius: 4px;
      background: #646cff;
      color: #fff;
      cursor: pointer;
    }
    button:hover {
      background: #535bf2;
    }
  </style>
  