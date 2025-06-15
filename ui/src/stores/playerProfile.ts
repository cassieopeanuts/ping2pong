// ping2pong/ui/src/stores/playerProfile.ts
import { writable } from "svelte/store";
import type { AgentPubKey, AppClient, Record, Entry } from "@holochain/client"; // Import AppClient, Record, Entry
import { decode } from "@msgpack/msgpack"; // Import decode
import type { Player } from '../ping_2_pong/ping_2_pong/types'; // Assuming Player type path
import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from '../holochainConfig'; // Import config

export interface PlayerProfile {
  nickname: string;
  agentKey: AgentPubKey;
}

// Initialize with null or load from storage if implementing persistence
export const playerProfile = writable<PlayerProfile | null>(null);

export async function checkAndLoadExistingProfile(client: AppClient): Promise<boolean> {
  if (!client) {
    console.error("[playerProfile] Client not provided to checkAndLoadExistingProfile.");
    playerProfile.set(null);
    return false;
  }

  try {
    const myPubKey = client.myPubKey;
    // console.log("[playerProfile] Checking for existing profile for agent:", encodeHashToBase64(myPubKey));

    const record: Record | null = await client.callZome({
      cap_secret: null,
      role_name: HOLOCHAIN_ROLE_NAME,
      zome_name: HOLOCHAIN_ZOME_NAME,
      fn_name: "get_player_profile_by_agent_key", // This is the function used by profilesStore
      payload: myPubKey,
    });

    if (record && record.entry && (record.entry as any).Present) {
      const entry = (record.entry as any).Present.entry as Uint8Array;
      const fetchedPlayer = decode(entry) as Player; // Assuming Player is { player_name: string, player_key: AgentPubKey }

      if (fetchedPlayer && fetchedPlayer.player_name) {
        // console.log("[playerProfile] Profile found:", fetchedPlayer);
        playerProfile.set({
          nickname: fetchedPlayer.player_name,
          agentKey: fetchedPlayer.player_key, // Ensure this is the correct AgentPubKey object
        });
        return true;
      } else {
        // console.log("[playerProfile] Profile record found, but data format incorrect or player_name missing.", fetchedPlayer);
        playerProfile.set(null);
        return false;
      }
    } else {
      // console.log("[playerProfile] No profile record found for agent:", encodeHashToBase64(myPubKey));
      playerProfile.set(null);
      return false;
    }
  } catch (e) {
    // console.error("[playerProfile] Error fetching profile for agent:", e);
    // Check if the error is because the player doesn't exist (e.g., "Player profile not found" or similar)
    // This depends on how the zome call errors out for non-existent profiles.
    // If it's a "not found" type of error, it's not a critical failure, just means no profile.
    // For other errors (network, etc.), it's more problematic.
    // For simplicity now, any error leads to no auto-login.
    playerProfile.set(null);
    return false;
  }
}