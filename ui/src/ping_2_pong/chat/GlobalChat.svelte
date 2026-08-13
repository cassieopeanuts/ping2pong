<script lang="ts">
  import { onMount, getContext, onDestroy } from 'svelte';
  import { globalChatMessages, addChatMessage, clearChatMessages } from '../../stores/chatStore';
  import { clientContext, type ClientContext } from '../../contexts'; // Added ClientContext for typing
  import type { AppClient, AgentPubKeyB64 } from '@holochain/client'; // Added AgentPubKeyB64
  import { encodeHashToBase64 } from '@holochain/client';
  import { truncatePubkey } from '../../utils';
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from '../../holochainConfig';
  import { writable, get as getStoreValue } from 'svelte/store'; // Added Svelte store imports
  import { getOrFetchProfile, profilesCache, type DisplayProfile } from '../../stores/profilesStore'; // Import profile store

  let messageContent: string = "";
  let chatBox: HTMLElement; // For auto-scrolling
  let unsubscribeFromStore: (() => void) | undefined;

  let sendError: string | null = null;
  let isSending: boolean = false;

  let client: AppClient; // To be initialized in onMount
  const appClientContext = getContext<ClientContext>(clientContext); // Typed getContext

  function getSenderNickname(senderB64: string): string {
    const cached = getStoreValue(profilesCache).get(senderB64);
    if (cached && cached.nickname) return cached.nickname;
    return truncatePubkey(senderB64, 4, 4);
  }

  async function sendMessage() {
    if (!messageContent.trim()) return;
    
    if (!client) { // Check if the module-level client is initialized
      sendError = "Holochain client not ready. Please wait or refresh.";
      console.error("sendMessage called before client was initialized.");
      return;
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

  let isMounted = true;
  let scrollTimeoutId: ReturnType<typeof setTimeout> | undefined;

  // Basic auto-scroll
  function scrollToBottom() {
    if (chatBox && isMounted) {
      // Use requestAnimationFrame to wait for DOM updates before scrolling
      requestAnimationFrame(() => {
        if (chatBox && isMounted) {
          chatBox.scrollTop = chatBox.scrollHeight;
        }
      });
    }
  }

  onMount(async () => {
    const fetchedClient = await appClientContext.getClient();
    if (!isMounted) return;
    client = fetchedClient;

    // Load persisted chat history from DHT chain
    try {
      const historySignals: any[] = await client.callZome({
        cap_secret: null,
        role_name: HOLOCHAIN_ROLE_NAME,
        zome_name: HOLOCHAIN_ZOME_NAME,
        fn_name: "get_latest_chat_messages",
        payload: null,
      });

      if (isMounted && Array.isArray(historySignals)) {
        clearChatMessages();
        historySignals.forEach((sig) => {
          let s = sig;
          if (sig?.type === "GlobalChatMessage") {
            const sender = s.sender;
            const content = s.content;
            const timestamp = s.timestamp;

            let senderB64 = "";
            if (typeof sender === "string") {
                senderB64 = sender;
            } else if (sender) {
                try { senderB64 = encodeHashToBase64(sender); } catch(e) { senderB64 = String(sender); }
            }

            let messageTimestamp = Date.now();
            if (typeof timestamp === "number") {
                messageTimestamp = timestamp > 1e12 ? Math.floor(timestamp / 1000) : timestamp;
            } else if (Array.isArray(timestamp) && timestamp.length >= 1) {
                messageTimestamp = timestamp[0] * 1000 + Math.floor((timestamp[1] || 0) / 1000000);
            }

            if (content && typeof content === "string") {
                addChatMessage({
                    type: "GlobalChatMessage",
                    sender: senderB64,
                    content: content,
                    timestamp: messageTimestamp,
                });
            }
          }
        });
      }
    } catch (e) {
      console.warn("Could not load chat history from chain:", e);
    }

    unsubscribeFromStore = globalChatMessages.subscribe((messages) => {
      if (!isMounted) return;
      if (messages.length > 0) {
        scrollToBottom();
        const lastMsg = messages[messages.length - 1];
        if (lastMsg && lastMsg.sender && client) {
          getOrFetchProfile(client, lastMsg.sender);
        }
      }
    });
    scrollTimeoutId = setTimeout(scrollToBottom, 50);
  });

  onDestroy(() => {
    isMounted = false;
    if (unsubscribeFromStore) {
      unsubscribeFromStore();
    }
    if (scrollTimeoutId) {
      clearTimeout(scrollTimeoutId);
    }
  });

  // Helper to truncate sender pubkey for display - REMOVED, using imported version
  // function truncatePubkey(pubkey: string): string {
  //   if (!pubkey || typeof pubkey !== 'string') return "anonymous"; // Handle undefined or non-string pubkeys
  //   // Assuming pubkey is Base64. A typical Holochain AgentPubKey (uCA...) is longer.
  //   // Adjust slicing if needed based on actual pubkey format and length.
  //   const prefixLength = 8;
  //   const suffixLength = 6;
  //   if (pubkey.length <= prefixLength + suffixLength + 3) return pubkey; // Don't truncate if too short
  //   return pubkey.slice(0, prefixLength) + "..." + pubkey.slice(-suffixLength);
  // }

</script>

<div class="global-chat-placeholder">
  <h4>Global Chat</h4>
  <div class="chat-messages-placeholder" bind:this={chatBox}>
    {#each $globalChatMessages as msg (msg.timestamp.toString() + msg.sender)}
      <p>
        <span title={msg.sender} class="sender">
          {getSenderNickname(msg.sender)}:
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
    font-size: 1.25rem; /* 20px */
    /* Consider line-height if it looks off, global h4 line-height is not set in index.css directly, inherits from h1,h2,h3 which is 1.3 for h3 */
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
