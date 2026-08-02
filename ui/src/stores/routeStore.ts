import { writable } from "svelte/store";

// Define a simple store for the current route.
// Possible routes: "dashboard", "gameplay", "statistics"
export const currentRoute = writable("dashboard");