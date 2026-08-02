// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/score.rs
use hdk::prelude::*;

// Score entry, recorded at the end of a game for one player.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Score {
    pub game_id: ActionHash, // Links back to the original Game create action
    pub player: AgentPubKey, // The player this score belongs to
    pub player_points: u32,  // Points scored by this player in the game
    pub created_at: Timestamp, // When the score was recorded
                             // pub opponent_points: u32, // Optional: Could store opponent's score too
                             // pub game_outcome: GameOutcome, // Optional: Win/Loss/Draw enum?
}