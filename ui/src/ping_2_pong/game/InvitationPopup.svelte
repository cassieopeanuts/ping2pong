<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { decodeHashFromBase64 } from '@holochain/client';

  export let inviter: string;           // already base-64
  export let gameId: string;            // base-64 **string**
  export let error: string | null = null; // New error prop

  const dispatch = createEventDispatcher();

  /** user clicks ✅ */
  function acceptInvitation () {
    // convert the base-64 string back to the ActionHash bytes
    const hash = decodeHashFromBase64(gameId);
    dispatch('accept', { gameId: hash });   // <-- detail *is* the raw hash
  }

  /** user clicks ❌ */
  function declineInvitation () {
    dispatch('decline');                    // no payload needed
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
  /* Styles from index.css are already applied via classes like .invitation-popup */
  /* We only need to override specific font sizes if the defaults don't look good. */

  h3 {
    font-size: 1.25rem; /* 20px. Default in index.css for .invitation-popup h3 is 1.8em (28.8px) via global h3 */
    line-height: 1.2;   /* Global h3 line-height is 1.3, this makes it a bit tighter */
  }

  p { /* This targets the main descriptive paragraph */
    font-size: 1rem;    /* 16px. Default in index.css for .invitation-popup p is 0.95em (15.2px) */
    line-height: 1.3;   /* Global p line-height is 1.6, this is tighter for the popup */
                        /* Note: .error-message already has 1em (16px) from global styles, which is fine. */
  }

  button {
    font-size: 1rem;    /* 16px. Default in index.css for .invitation-popup button is 0.95em (15.2px) */
    /* Padding will scale based on this new font size due to em units in global style */
  }
</style>