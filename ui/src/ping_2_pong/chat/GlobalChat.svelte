<script lang="ts">
  import { onMount, getContext, onDestroy } from 'svelte';
  import { globalChatMessages } from '../../stores/chatStore';
  import { clientContext, type ClientContext } from '../../contexts'; 
  import type { AppClient, AgentPubKeyB64 } from '@holochain/client'; 
  import { truncatePubkey } from '../../utils';
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from '../../holochainConfig';
  import { writable, get as getStoreValue } from 'svelte/store'; 
  import { getOrFetchProfile, type DisplayProfile } from '../../stores/profilesStore'; 

  let messageContent: string = "";
  let chatBox: HTMLElement; // For auto-scrolling
  let unsubscribeFromStore: (() => void) | undefined;

  let sendError: string | null = null;
  let isSending: boolean = false;

  let client: AppClient; // To be initialized in onMount
  const appClientContext = getContext<ClientContext>(clientContext); // Typed getContext

  // Store for fetched sender profiles
  let senderProfiles = writable<Map<AgentPubKeyB64, DisplayProfile | null>>(new Map());

  // Reactive block to fetch profiles for new senders
  $: if ($globalChatMessages && client) {
    const currentProfiles = getStoreValue(senderProfiles);
    for (const msg of $globalChatMessages) {
      if (!currentProfiles.has(msg.sender)) {
        // Set to null initially to indicate loading / prevent multiple fetches
        senderProfiles.update(m => {
          const newMap = new Map(m);
          newMap.set(msg.sender, null);
          return newMap;
        });
        getOrFetchProfile(client, msg.sender).then(profile => {
          if (profile) {
            senderProfiles.update(m => {
              const newMap = new Map(m);
              newMap.set(msg.sender, profile);
              return newMap;
            });
          }
          // If profile is null (error or not found), it remains null in the map,
          // which will cause fallback to truncatePubkey in the template.
        });
      }
    }
  }

  async function sendMessage() {
    if (!messageContent.trim()) return;
    
    if (!client) { // Check if the module-level client is initialized
      sendError = "Holochain client not ready. Please wait or refresh.";
      console.error("sendMessage called before client was initialized.");
      return; // Do not proceed if client is not ready
    }

    isSending = true;
    sendError = null;
    try {
      await client.callZome({
        cap_secret: null,
        role_name: HOLOCHAIN_ROLE_NAME,
        zome_name: HOLOCHAIN_ZOME_NAME,
        fn_name: "send_global_chat_message",
        payload: messageContent,
      });
      messageContent = ""; // Clear message content on success
    } catch (e: any) {
      console.error("Error sending chat message:", e);
      sendError = e.data?.data || e.message || "Failed to send message. Please try again.";
    } finally {
      isSending = false;
    }
  }

  // Clear error when user starts typing
  $: if (messageContent && sendError) {
    sendError = null;
  }

  function formatTimestamp(timestamp: number): string {
    if (!timestamp) return ""; // Handle cases where timestamp might be undefined or 0
    return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }

  // Basic auto-scroll
  function scrollToBottom() {
    if (chatBox) {
      // Use requestAnimationFrame to wait for DOM updates before scrolling
      requestAnimationFrame(() => {
        chatBox.scrollTop = chatBox.scrollHeight;
      });
    }
  }

  onMount(async () => { // Made onMount async
    client = await appClientContext.getClient(); // Initialize client

    // Scroll to bottom when component mounts and when messages change
    unsubscribeFromStore = globalChatMessages.subscribe((messages) => {
      if (messages.length > 0) { // Only scroll if there are messages
        scrollToBottom();
      }
    });
    // Initial scroll attempt, useful if messages are already loaded
    // Ensure chatBox is rendered before scrolling
    setTimeout(scrollToBottom, 50);
  });

  onDestroy(() => {
    if (unsubscribeFromStore) {
      unsubscribeFromStore();
    }
  });
</script>

<div class="global-chat-placeholder">
  <h4>Global Chat</h4>
  <div class="chat-messages-placeholder" bind:this={chatBox}>
    {#each $globalChatMessages as msg (msg.timestamp.toString() + msg.sender)}
      {@const profile = $senderProfiles.get(msg.sender)}
      <p>
        <span title={msg.sender} class="sender"> <!-- Added class="sender" for consistent styling if needed -->
          {profile?.nickname || truncatePubkey(msg.sender, 4, 4)}:
        </span>
        <!-- Message content will be styled by '.chat-messages-placeholder p' -->
        {msg.content}
        <span class="chat-timestamp">{formatTimestamp(msg.timestamp)}</span>
      </p>
    {:else}
      <!-- This paragraph will inherit styles from '.chat-messages-placeholder p' and can be centered with a utility class if needed -->
      <p class="text-center"> 
        No messages yet. Be the first to say something!
      </p>
    {/each}
  </div>
   <!-- Form styled to lay out input and button horizontally, using global styles for elements -->
   <form on:submit|preventDefault={sendMessage} style="display: flex; flex-direction: column; gap: 8px; margin-top: 1rem;">
     <div style="display: flex; gap: 8px; align-items: center;">
       <input type="text" bind:value={messageContent} placeholder="Type a message..." aria-label="Chat message input" style="flex-grow: 1; margin: 0;" disabled={isSending} />
       <button type="submit" disabled={isSending}>
         {#if isSending}Sending...{:else}Send{/if}
       </button>
     </div>
     {#if sendError}
       <p class="error-message" style="margin: 0.5rem 0 0 0; padding: 0.5em;">{sendError}</p>
     {/if}
   </form>
</div>

<style>
  h4 {
    font-size: 1.25rem; 
    line-height: 1.3;
  }

  .chat-messages-placeholder p {
    font-size: 0.75rem; /* 12px */
    line-height: 1.3; /* Adjusted for better readability with 'Press Start 2P' */
    word-wrap: break-word; /* Ensure long messages without spaces wrap */
    overflow-wrap: break-word; /* Modern equivalent for word-wrap */
  }

  .chat-messages-placeholder .sender {
    /* font-size is inherited from p, which is now 0.75rem */
    /* No specific font-size change needed here unless it should be different from message content */
  }

  .chat-timestamp {
    font-size: 0.75rem; /* 12px, same as chat message p, differentiated by color */
    /* display: inline-block; */ /* Ensure it flows with text but can have margin if needed */
    /* margin-left: 0.5rem; */ /* Re-evaluate if needed, index.css had 8px */
  }

  /* Input and button will inherit 1em (16px) from global styles, which is fine. */
  /* Error message will inherit 1em (16px) from global .error-message style, which is fine. */
</style>
