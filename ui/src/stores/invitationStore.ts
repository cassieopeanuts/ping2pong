    // src/stores/invitationStore.ts
    import { writable } from "svelte/store";
    import type { GameInvitationSignal } from "../ping_2_pong/ping_2_pong/types"; // Adjust path if needed
    import { encodeHashToBase64 } from "@holochain/client"; // Import for logging

    // Store an array of received invitations
    export const invitations = writable<GameInvitationSignal[]>([]);

    // Helper function to add an invitation only if it's not already present
    export function addInvitation(newInvitation: GameInvitationSignal) {
        // console.log("[invitationStore] Attempting to add invitation:", {
        //     game_id: encodeHashToBase64(newInvitation.game_id),
        //     inviter: encodeHashToBase64(newInvitation.inviter)
        // });
        invitations.update(currentInvitations => {
            const exists = currentInvitations.some(inv =>
                // Compare game IDs using Base64 strings for reliable comparison
                encodeHashToBase64(inv.game_id) === encodeHashToBase64(newInvitation.game_id)
            );
            if (!exists) {
                // console.log("[invitationStore] Invitation does not exist, adding.");
                return [...currentInvitations, newInvitation];
            }
            // console.log("[invitationStore] Invitation already exists, not adding again.");
            return currentInvitations; // Return unchanged array if exists
        });
    }

    // Helper function to remove an invitation (e.g., after accepting/declining)
    export function removeInvitation(gameIdToRemove: Uint8Array) { // Accept ActionHash (Uint8Array)
        const gameIdB64 = encodeHashToBase64(gameIdToRemove);
        // console.log("[invitationStore] Attempting to remove invitation with gameId:", gameIdB64);
        invitations.update(currentInvitations => {
            const initialLength = currentInvitations.length;
            const filtered = currentInvitations.filter(inv =>
                encodeHashToBase64(inv.game_id) !== gameIdB64
            );
            if (filtered.length < initialLength) {
                // console.log("[invitationStore] Invitation removed.");
            } else {
                // console.log("[invitationStore] Invitation to remove not found.");
            }
            return filtered;
        });
    }
    