// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/signals.rs
use hdk::prelude::*;
// Import the Signal enum definition from the main library file (lib.rs)
use crate::Signal;

// --- Payload Structs for Signal Functions ---
// These structs define the data expected from the UI when calling the signal sending functions.

#[derive(Serialize, Deserialize, Debug)]
pub struct PaddleUpdatePayload {
    pub game_id: ActionHash, // Identifies the game this update belongs to
    pub paddle_y: u32,       // The new Y-coordinate of the paddle
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BallUpdatePayload {
     pub game_id: ActionHash, // Identifies the game
     pub ball_x: u32,         // Ball's X-coordinate
     pub ball_y: u32,         // Ball's Y-coordinate
     // Use i32 for velocity components as they can be negative
     pub ball_dx: i32,        // Ball's velocity in X direction
     pub ball_dy: i32,        // Ball's velocity in Y direction
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GameOverPayload {
     pub game_id: ActionHash,      // Identifies the game
     pub winner: Option<AgentPubKey>, // The public key of the winner, or None (e.g., if draw or canceled)
     pub score1: u32,              // Final score for player 1
     pub score2: u32,              // Final score for player 2
}


// --- Extern Functions for Sending Specific Game Signals ---

/// Sends a signal indicating a paddle position update.
/// The 'player' field in the emitted signal is automatically set to the caller's public key.
#[hdk_extern]
pub fn send_paddle_update(payload: PaddleUpdatePayload) -> ExternResult<()> {
    // Get the public key of the agent calling this function
    let my_pub_key = agent_info()?.agent_latest_pubkey;

    // TODO Optional Validation:
    // You could add a check here to ensure the caller (`my_pub_key`)
    // is actually a participant in the game identified by `payload.game_id`.
    // This requires fetching the game state (`get_latest_game`) which adds overhead.
    // Example (pseudo-code, needs proper implementation if desired):
    // let game = get_latest_game(payload.game_id)?;
    // if game.player_1 != my_pub_key && game.player_2 != Some(my_pub_key) {
    //     return Err(wasm_error!("Caller is not part of the specified game"));
    // }

    // Construct the specific Signal enum variant
    let signal = Signal::PaddleUpdate {
        game_id: payload.game_id,
        player: my_pub_key, // The player sending the update is the caller
        paddle_y: payload.paddle_y,
    };

    // Emit the signal to other participants (network implementation specific)
    emit_signal(&signal)
}

/// Sends a signal indicating a ball position and velocity update.
/// Typically called only by Player 1.
#[hdk_extern]
pub fn send_ball_update(payload: BallUpdatePayload) -> ExternResult<()> {
    // TODO Optional Validation:
    // You could add a check here to ensure the caller is Player 1
    // of the game identified by `payload.game_id`.
    // Example (pseudo-code):
    // let game = get_latest_game(payload.game_id)?;
    // if game.player_1 != agent_info()?.agent_latest_pubkey {
    //     return Err(wasm_error!("Only Player 1 can send ball updates"));
    // }

    // Construct the specific Signal enum variant
    let signal = Signal::BallUpdate {
        game_id: payload.game_id,
        ball_x: payload.ball_x,
        ball_y: payload.ball_y,
        ball_dx: payload.ball_dx,
        ball_dy: payload.ball_dy,
    };

    // Emit the signal
    emit_signal(&signal)
}

/// Sends a signal indicating the game has ended, including the winner and final scores.
/// Typically called only by Player 1 after detecting a win condition.
#[hdk_extern]
pub fn send_game_over(payload: GameOverPayload) -> ExternResult<()> {
    // TODO Optional Validation:
    // Check if the caller is Player 1 (or a participant) of the game.

    // Construct the specific Signal enum variant
     let signal = Signal::GameOver {
        game_id: payload.game_id,
        winner: payload.winner, // Pass the Option<AgentPubKey> directly
        score1: payload.score1,
        score2: payload.score2,
    };

    // Emit the signal
    emit_signal(&signal)
}
