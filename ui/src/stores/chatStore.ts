import { writable } from 'svelte/store';
import type { GlobalChatMessageSignal } from '../ping_2_pong/ping_2_pong/types'; // Path seems correct based on previous step

const MAX_CHAT_MESSAGES = 100; // Define a maximum number of messages to store

export const globalChatMessages = writable<GlobalChatMessageSignal[]>([]);

export function addChatMessage(newMessage: GlobalChatMessageSignal) {
    globalChatMessages.update(messages => {
        const updatedMessages = [...messages, newMessage];
        if (updatedMessages.length > MAX_CHAT_MESSAGES) {
            // Remove the oldest message(s) to maintain the cap
            return updatedMessages.slice(updatedMessages.length - MAX_CHAT_MESSAGES);
        }
        return updatedMessages;
    });
    // For debugging:
    // console.log('[chatStore] Added message:', newMessage);
    // globalChatMessages.subscribe(value => console.log('[chatStore] Current messages:', value))();
}

// Optional: Function to clear chat messages if needed
export function clearChatMessages() {
    globalChatMessages.set([]);
}
