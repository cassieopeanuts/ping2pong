// ui/src/stores/profilesStore.ts
import { writable, get as getStoreValue } from 'svelte/store';
import type { AppClient, AgentPubKey, Record, Entry, ActionHash, AgentPubKeyB64 } from '@holochain/client'; // Added AgentPubKeyB64
import { encodeHashToBase64, decodeHashFromBase64 } from '@holochain/client'; // Added decodeHashFromBase64
import { decode } from '@msgpack/msgpack';
import type { Player } from '../ping_2_pong/ping_2_pong/types'; // Corrected path assuming types.ts is in ping_2_pong/ping_2_pong
import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from '../holochainConfig';

export interface DisplayProfile {
  nickname: string;
  agentKeyB64: string;
}

const profilesCache = writable<Map<AgentPubKeyB64, DisplayProfile>>(new Map());
const fetchingStatus = writable<Map<AgentPubKeyB64, boolean>>(new Map()); // To prevent concurrent fetches

// Function to get a profile from cache or fetch if not present
export async function getOrFetchProfile(client: AppClient, agentKeyToFetch: AgentPubKey | AgentPubKeyB64): Promise<DisplayProfile | null> {
  const agentKeyB64 = typeof agentKeyToFetch === 'string' ? agentKeyToFetch : encodeHashToBase64(agentKeyToFetch);

  const currentCache = getStoreValue(profilesCache);
  if (currentCache.has(agentKeyB64)) {
    return currentCache.get(agentKeyB64)!;
  }

  const isFetching = getStoreValue(fetchingStatus).get(agentKeyB64);
  if (isFetching) {
    // Optional: wait for the ongoing fetch to complete, or return null/stale immediately
    // For simplicity, returning null here, component can retry or show loading.
    // A more advanced version could return a promise that resolves when the ongoing fetch completes.
    console.log(`[profilesStore] Already fetching profile for ${agentKeyB64}, returning null for now.`);
    return null;
  }

  fetchingStatus.update(s => s.set(agentKeyB64, true));
  console.log(`[profilesStore] Fetching profile for ${agentKeyB64}`);

  try {
    const record: Record | null = await client.callZome({
      cap_secret: null,
      role_name: HOLOCHAIN_ROLE_NAME,
      zome_name: HOLOCHAIN_ZOME_NAME,
      fn_name: "get_player_profile_by_agent_key",
      payload: typeof agentKeyToFetch === 'string' ? decodeHashFromBase64(agentKeyToFetch) : agentKeyToFetch, // Ensure payload is AgentPubKey
    });

    if (record && record.entry && (record.entry as any).Present) {
      const entry = (record.entry as any).Present.entry as Uint8Array;
      const player = decode(entry) as Player; // Player type from integrity zome {player_name, player_key}

      const displayProfile: DisplayProfile = {
        nickname: player.player_name,
        agentKeyB64: encodeHashToBase64(player.player_key), // Should match agentKeyB64 from input if it was AgentPubKey
      };

      // Verify consistency if original was AgentPubKey
      if (typeof agentKeyToFetch !== 'string' && agentKeyB64 !== displayProfile.agentKeyB64) {
        console.warn(`[profilesStore] Mismatch between fetched agentKeyB64 ${displayProfile.agentKeyB64} and input agentKeyB64 ${agentKeyB64}`);
        // Potentially handle this error, e.g., by not caching or using the input key for caching.
        // For now, we'll trust the fetched data but log a warning.
      }

      profilesCache.update(cache => {
        const newCache = new Map(cache);
        newCache.set(agentKeyB64, displayProfile); // Use the consistent agentKeyB64 derived from input
        return newCache;
      });
      console.log(`[profilesStore] Fetched and cached profile for ${agentKeyB64}:`, displayProfile);
      fetchingStatus.update(s => s.set(agentKeyB64, false));
      return displayProfile;
    } else {
      console.log(`[profilesStore] No profile record found for ${agentKeyB64}.`);
      fetchingStatus.update(s => s.set(agentKeyB64, false));
      return null;
    }
  } catch (e) {
    console.error(`[profilesStore] Error fetching profile for ${agentKeyB64}:`, e);
    fetchingStatus.update(s => s.set(agentKeyB64, false));
    return null;
  }
}
