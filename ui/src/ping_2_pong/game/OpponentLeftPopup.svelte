<script lang="ts">
    import { createEventDispatcher } from 'svelte';
  
    export let opponentNickname: string = 'Opponent';
    export let opponentAgentKeyB64: string = ''; 
  
    const dispatch = createEventDispatcher();
  
    function handleDismiss() {
      dispatch('dismissed');
    }
  </script>
  
  <div class="popup-overlay">
    <div class="popup-content">
      <h3 title={`Agent: ${opponentAgentKeyB64}`}>Game Abandoned</h3>
      <p>{opponentNickname} has left the game.</p>
      <button on:click={handleDismiss}>Back to Lobby</button>
    </div>
  </div>
  
  <style>
    .popup-overlay {
      position: fixed;
      top: 0;
      left: 0;
      width: 100%;
      height: 100%;
      background-color: rgba(0, 0, 0, 0.6);
      display: flex;
      justify-content: center;
      align-items: center;
      z-index: 1000;
    }
  
    .popup-content {
      background-color: var(--secondary-bg-color, #333); 
      color: var(--primary-text-color, #fff); 
      padding: 2rem;
      border-radius: 8px;
      text-align: center;
      border: 2px solid var(--border-color, #FFA500); 
      box-shadow: 0 4px 15px rgba(0, 0, 0, 0.2);
    }
  
    .popup-content h3 {
      margin-top: 0;
      margin-bottom: 1rem;
      color: var(--primary-text-color, #FFA500); 
    }
  
    .popup-content p {
      margin-bottom: 1.5rem;
      font-size: 1rem;
    }
  
    .popup-content button {
      background-color: var(--accent-color, #FFA500); 
      color: var(--button-text-color, black); 
      border: none;
      padding: 0.75rem 1.5rem;
      border-radius: 5px;
      font-size: 1rem;
      cursor: pointer;
      transition: background-color 0.2s ease;
    }
  
    .popup-content button:hover {
      background-color: var(--accent-hover-color, #cc8400); 
    }
  </style>
  