// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/presence.rs
use hdk::prelude::*;

// Presence entry to indicate recent activity.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Presence {
    pub agent_pubkey: AgentPubKey, // The agent who is present
    pub timestamp: u64, // Milliseconds since epoch (client-generated, validated)
}