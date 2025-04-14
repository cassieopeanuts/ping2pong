// ping2pong/ui/src/stores/currentGame.ts
import { writable } from "svelte/store";
import type { ActionHash } from "@holochain/client"; // Import ActionHash

// Store for the active game's ORIGINAL ActionHash that the user is playing/joining.
// FIX: Change type from string | null to ActionHash | null
export const currentGame = writable<ActionHash | null>(null);