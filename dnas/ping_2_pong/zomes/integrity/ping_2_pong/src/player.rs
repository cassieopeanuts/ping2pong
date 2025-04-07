// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/player.rs
use hdk::prelude::*;

// Player profile entry.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Player {
    pub player_key: AgentPubKey, // The agent this profile belongs to
    pub player_name: String,     // Chosen nickname
                                 // Add other profile fields? Elo rating? Avatar URL?
                                 // pub elo_rating: Option<u32>,
}