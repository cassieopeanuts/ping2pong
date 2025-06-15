import { encodeHashToBase64, type AgentPubKey, type AgentPubKeyB64 } from "@holochain/client";

/**
 * Truncates a Holochain AgentPubKey (either raw or Base64 encoded) for display.
 *
 * @param pubkey - The public key to truncate. Can be AgentPubKey (Uint8Array),
 *                 AgentPubKeyB64 (string), a generic string, null, or undefined.
 * @param prefixLength - The number of characters to show from the beginning of the Base64 string.
 * @param suffixLength - The number of characters to show from the end of the Base64 string.
 * @returns A truncated string representation of the public key (e.g., "uCAm...") or "Invalid Key" / "N/A".
 */
export function truncatePubkey(
  pubkey: AgentPubKey | AgentPubKeyB64 | string | null | undefined,
  prefixLength: number = 8,
  suffixLength: number = 6
): string {
  if (pubkey === null || pubkey === undefined) {
    return "N/A";
  }

  let pubkeyB64: string;

  if (typeof pubkey === 'string') {
    // It's already a string, assume it's Base64 or a similar format
    pubkeyB64 = pubkey;
  } else if (pubkey instanceof Uint8Array) {
    // It's an AgentPubKey (Uint8Array), try to encode it
    try {
      pubkeyB64 = encodeHashToBase64(pubkey);
    } catch (e) {
      console.error("Error encoding AgentPubKey to Base64:", e);
      return "Invalid Key"; // Or some other error indicator
    }
  } else {
    // Should not happen with the given types, but as a fallback
    console.warn("Unknown pubkey type for truncation:", pubkey);
    return "Invalid Key";
  }

  if (pubkeyB64.length <= prefixLength + suffixLength + 3) { // +3 for "..."
    return pubkeyB64; // Don't truncate if it's already short enough
  }

  return `${pubkeyB64.slice(0, prefixLength)}...${pubkeyB64.slice(-suffixLength)}`;
}
