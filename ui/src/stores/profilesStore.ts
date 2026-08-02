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

export const profilesCache = writable<Map<AgentPubKeyB64, DisplayProfile>>(new Map());
const fetchingStatus = writable<Map<AgentPubKeyB64, boolean>>(new Map()); // To prevent concurrent fetches

export function cacheProfile(agentKeyToCache: AgentPubKey | AgentPubKeyB64, nickname: string) {
  const agentKeyB64 = typeof agentKeyToCache === 'string' ? agentKeyToCache : encodeHashToBase64(agentKeyToCache);
  if (!agentKeyB64 || !nickname) return;
  profilesCache.update(cache => {
    const existing = cache.get(agentKeyB64);
    if (existing && existing.nickname === nickname) return cache; // Skip redundant updates
    const newCache = new Map(cache);
    newCache.set(agentKeyB64, { nickname, agentKeyB64 });
    return newCache;
  });
}

// Function to get a profile from cache or fetch if not present
export async function getOrFetchProfile(client: AppClient, agentKeyToFetch: AgentPubKey | AgentPubKeyB64): Promise<DisplayProfile | null> {
  const agentKeyB64 = typeof agentKeyToFetch === 'string' ? agentKeyToFetch : encodeHashToBase64(agentKeyToFetch);

  const currentCache = getStoreValue(profilesCache);
  if (currentCache.has(agentKeyB64)) {
    return currentCache.get(agentKeyB64)!;
  }

  const isFetching = getStoreValue(fetchingStatus).get(agentKeyB64);
  if (isFetching) {
    return null;
  }

  fetchingStatus.update(s => {
    const newStatus = new Map(s);
    newStatus.set(agentKeyB64, true);
    return newStatus;
  });

  try {
    const record: Record | null = await client.callZome({
      cap_secret: null,
      role_name: HOLOCHAIN_ROLE_NAME,
      zome_name: HOLOCHAIN_ZOME_NAME,
      fn_name: "get_player_profile_by_agent_key",
      payload: typeof agentKeyToFetch === 'string' ? decodeHashFromBase64(agentKeyToFetch) : agentKeyToFetch,
    });

    if (record && record.entry && (record.entry as any).Present) {
      const entry = (record.entry as any).Present.entry as Uint8Array;
      const player = decode(entry) as Player;

      const displayProfile: DisplayProfile = {
        nickname: player.player_name,
        agentKeyB64: encodeHashToBase64(player.player_key),
      };

      profilesCache.update(cache => {
        const existing = cache.get(agentKeyB64);
        if (existing && existing.nickname === displayProfile.nickname) return cache;
        const newCache = new Map(cache);
        newCache.set(agentKeyB64, displayProfile);
        return newCache;
      });
      fetchingStatus.update(s => {
        const newStatus = new Map(s);
        newStatus.set(agentKeyB64, false);
        return newStatus;
      });
      return displayProfile;
    } else {
      fetchingStatus.update(s => {
        const newStatus = new Map(s);
        newStatus.set(agentKeyB64, false);
        return newStatus;
      });
      return null;
    }
  } catch (e) {
    fetchingStatus.update(s => {
      const newStatus = new Map(s);
      newStatus.set(agentKeyB64, false);
      return newStatus;
    });
    return null;
  }
}
