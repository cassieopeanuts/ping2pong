// src/stores/invitationStore.ts
import { writable } from "svelte/store";
import type { GameInvitationSignal } from "../ping_2_pong/ping_2_pong/types"; // Adjust path if needed

// Store an array of received invitations
export const invitations = writable<GameInvitationSignal[]>([]);

// Helper function to add an invitation only if it's not already present
export function addInvitation(newInvitation: GameInvitationSignal) {
    invitations.update(currentInvitations => {
        const exists = currentInvitations.some(inv =>
            // Compare game IDs (convert ActionHash to comparable string like Base64)
            JSON.stringify(inv.game_id) === JSON.stringify(newInvitation.game_id)
        );
        if (!exists) {
            return [...currentInvitations, newInvitation];
        }
        return currentInvitations; // Return unchanged array if exists
    });
}

// Helper function to remove an invitation (e.g., after accepting/declining)
export function removeInvitation(gameIdToRemove: Uint8Array) { // Accept ActionHash (Uint8Array)
    invitations.update(currentInvitations =>
        currentInvitations.filter(inv =>
            JSON.stringify(inv.game_id) !== JSON.stringify(gameIdToRemove)
        )
    );
}