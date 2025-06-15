use hdk::prelude::*;

// Define the Game Status enum.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum GameStatus {
    Waiting,    // Waiting for Player 2 or matchmaking
    InProgress, // Game actively being played
    Finished,   // Game concluded, score recorded/recordable
    Abandoned,  // Game terminated by a player exiting
}

// Game entry structure.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Game {
    pub player_1: AgentPubKey,
    pub player_2: Option<AgentPubKey>, 
    pub game_status: GameStatus,
    pub created_at: Timestamp,
    pub player_1_paddle: u32,
    pub player_2_paddle: u32,
    pub ball_x: u32,
    pub ball_y: u32,
}