use hdk::prelude::*;

// Score entry, recorded at the end of a game for one player.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Score {
    pub game_id: ActionHash, 
    pub player: AgentPubKey, 
    pub player_points: u32,  
    pub created_at: Timestamp, 
}