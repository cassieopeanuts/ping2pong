import { writable } from 'svelte/store';
import type { GlobalChatMessageSignal } from '../ping_2_pong/ping_2_pong/types';

const MAX_CHAT_MESSAGES = 100; // Define a maximum number of messages to store

export const globalChatMessages = writable<GlobalChatMessageSignal[]>([]);

export function addChatMessage(newMessage: GlobalChatMessageSignal) {
    globalChatMessages.update(messages => {
        // Prevent duplicate message additions (same sender, same content, within 2 seconds)
        const isDuplicate = messages.some(
            m => m.sender === newMessage.sender &&
                 m.content === newMessage.content &&
                 Math.abs((m.timestamp || 0) - (newMessage.timestamp || 0)) < 2000
        );
        if (isDuplicate) return messages;

        const updatedMessages = [...messages, newMessage];
        if (updatedMessages.length > MAX_CHAT_MESSAGES) {
            return updatedMessages.slice(updatedMessages.length - MAX_CHAT_MESSAGES);
        }
        return updatedMessages;
    });
}

export function clearChatMessages() {
    globalChatMessages.set([]);
}
