use hdk::prelude::*;

// Player profile entry.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Player {
    pub player_key: AgentPubKey, 
    pub player_name: String,     
}