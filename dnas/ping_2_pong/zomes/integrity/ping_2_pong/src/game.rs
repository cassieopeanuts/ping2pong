// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/game.rs
use hdk::prelude::*;

// Define the Game Status enum.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum GameStatus {
    Waiting,    // Waiting for Player 2 or matchmaking
    InProgress, // Game actively being played
    Finished,   // Game concluded, score recorded/recordable
    // Canceled? // Optional status
}

// Define the Game entry structure.
// Note: Paddle/Ball positions here are informational defaults or latest *saved* state,
// not the real-time state which is handled by signals.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct Game {
    pub player_1: AgentPubKey,
    pub player_2: Option<AgentPubKey>, // Optional initially
    pub game_status: GameStatus,
    pub created_at: Timestamp,
    // Informational / Default positions - not updated via DHT entry updates during gameplay
    pub player_1_paddle: u32,
    pub player_2_paddle: u32,
    pub ball_x: u32,
    pub ball_y: u32,
    // pub initial_ball_vector_x: i32, // Maybe store initial vector? Optional.
    // pub initial_ball_vector_y: i32,
}