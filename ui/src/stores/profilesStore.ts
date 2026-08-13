// ui/src/stores/profilesStore.ts
import { writable, get as getStoreValue } from 'svelte/store';
import type { AppClient, AgentPubKey, Record, AgentPubKeyB64 } from '@holochain/client';
import { encodeHashToBase64, decodeHashFromBase64 } from '@holochain/client';
import { decode } from '@msgpack/msgpack';
import type { Player } from '../ping_2_pong/ping_2_pong/types';
import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from '../holochainConfig';

export interface DisplayProfile {
  nickname: string;
  agentKeyB64: string;
}

export const profilesCache = writable<Map<AgentPubKeyB64, DisplayProfile>>(new Map());
const fetchingSet = new Set<AgentPubKeyB64>();
const negativeCacheMap = new Map<AgentPubKeyB64, number>(); // key -> timestamp

export function cacheProfile(agentKeyToCache: AgentPubKey | AgentPubKeyB64, nickname: string) {
  const agentKeyB64 = typeof agentKeyToCache === 'string' ? agentKeyToCache : encodeHashToBase64(agentKeyToCache);
  if (!agentKeyB64 || !nickname) return;
  negativeCacheMap.delete(agentKeyB64);
  profilesCache.update(cache => {
    const existing = cache.get(agentKeyB64);
    if (existing && existing.nickname === nickname) return cache;
    const newCache = new Map(cache);
    newCache.set(agentKeyB64, { nickname, agentKeyB64 });
    return newCache;
  });
}

// Function to get a profile from cache or fetch if not present
export async function getOrFetchProfile(client: AppClient, agentKeyToFetch: AgentPubKey | AgentPubKeyB64, forceFetch = false): Promise<DisplayProfile | null> {
  const agentKeyB64 = typeof agentKeyToFetch === 'string' ? agentKeyToFetch : encodeHashToBase64(agentKeyToFetch);

  const currentCache = getStoreValue(profilesCache);
  if (!forceFetch && currentCache.has(agentKeyB64)) {
    return currentCache.get(agentKeyB64)!;
  }

  const negTime = negativeCacheMap.get(agentKeyB64);
  if (!forceFetch && negTime && Date.now() - negTime < 3000) { // 3s negative cache
    return null;
  }

  if (fetchingSet.has(agentKeyB64)) {
    return null;
  }

  fetchingSet.add(agentKeyB64);

  try {
    const record: Record | null = await client.callZome({
      cap_secret: null,
      role_name: HOLOCHAIN_ROLE_NAME,
      zome_name: HOLOCHAIN_ZOME_NAME,
      fn_name: "get_player_profile_by_agent_key",
      payload: typeof agentKeyToFetch === 'string' ? decodeHashFromBase64(agentKeyToFetch) : agentKeyToFetch,
    });

    if (record && record.entry) {
      const presentEntry = (record.entry as any).Present || (record.entry as any).present || record.entry;
      const rawBytes = presentEntry.entry || presentEntry;
      if (rawBytes) {
        const player = decode(rawBytes) as Player;

        if (player && player.player_name) {
          const displayProfile: DisplayProfile = {
            nickname: player.player_name,
            agentKeyB64: encodeHashToBase64(player.player_key),
          };

          negativeCacheMap.delete(agentKeyB64);
          profilesCache.update(cache => {
            const newCache = new Map(cache);
            newCache.set(agentKeyB64, displayProfile);
            return newCache;
          });
          fetchingSet.delete(agentKeyB64);
          return displayProfile;
        }
      }
    }
    
    negativeCacheMap.set(agentKeyB64, Date.now());
    fetchingSet.delete(agentKeyB64);
    return null;
  } catch (e) {
    negativeCacheMap.set(agentKeyB64, Date.now());
    fetchingSet.delete(agentKeyB64);
    return null;
  }
}
