<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { decodeHashFromBase64 } from '@holochain/client';

  export let inviter: string;           
  export let gameId: string;            
  export let error: string | null = null; 

  const dispatch = createEventDispatcher();

  /** user clicks ✅ */
  function acceptInvitation () {
    // convert the base-64 string back to the ActionHash bytes
    const hash = decodeHashFromBase64(gameId);
    dispatch('accept', { gameId: hash });   
  }

  /** user clicks ❌ */
  function declineInvitation () {
    dispatch('decline');                    
  }
</script>

  <div class="invitation-popup">
    <h3>Game Invitation</h3>
    <p>You have been invited by <strong>{inviter}</strong> to join a game.</p>
    {#if error}
      <p class="error-message" style="margin-top: 0.5rem; margin-bottom: 0.5rem;">{error}</p>
    {/if}
    <div class="invitation-popup-buttons">
      <button on:click={acceptInvitation}>Accept</button>
      <button on:click={declineInvitation}>Decline</button>
    </div>
  </div>

<style>

  h3 {
    font-size: 1.25rem; 
    line-height: 1.2;   
  }

  p { 
    font-size: 1rem;    
    line-height: 1.3;   
  }

  button {
    font-size: 1rem;   
  }
</style>