import { writable } from "svelte/store";

// Store for the active game hash (or record) that the user is joining.
export const currentGame = writable<string | null>(null);
