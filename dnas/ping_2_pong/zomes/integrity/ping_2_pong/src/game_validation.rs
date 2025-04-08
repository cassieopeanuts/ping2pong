// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/game_validation.rs
use hdk::prelude::*;
use crate::game::{Game, GameStatus};
// Use core::time::Duration for stability if hdk::prelude::Duration is problematic
use core::time::Duration;
// Import Add/Sub traits for Timestamp arithmetic
use std::ops::{Add, Sub};

// Validate creation of a Game entry.
pub fn validate_create_game(
    action: &SignedActionHashed,
    game: Game,
) -> ExternResult<ValidateCallbackResult> {
    // 1. Check Author: Ensure the creator is Player 1 or Player 2 (if specified).
    let author = action.action().author();
    // Allow Player 2 to create only if they are specified in the entry
    if game.player_1 != *author && game.player_2.as_ref() != Some(author) {
         return Ok(ValidateCallbackResult::Invalid(
             "Game creator must be Player 1 or Player 2 specified in the entry".to_string(),
         ));
    }

    // 2. Check Initial Status: Must be 'Waiting'.
    if game.game_status != GameStatus::Waiting {
        return Ok(ValidateCallbackResult::Invalid(
            "Game must be created with 'Waiting' status".to_string(),
        ));
    }

     // 3. Check Player 1 != Player 2
     if let Some(p2) = &game.player_2 {
         if game.player_1 == *p2 {
             return Ok(ValidateCallbackResult::Invalid(
                 "Player 1 and Player 2 cannot be the same agent".to_string(),
             ));
         }
     }

     // 4. Check Timestamp plausibility (within reason, e.g., +/- 5 mins from action time)
     let action_time = action.action().timestamp();
     let five_minutes = Duration::from_secs(300);

     // Perform subtraction and map error
     let lower_bound = action_time.sub(five_minutes)
         .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp subtraction error: {}", e))))?;

     // Perform addition and map error
     let upper_bound = action_time.add(five_minutes)
         .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp addition error: {}", e))))?;

     // Now perform the comparison with the successfully unwrapped Timestamps
     if game.created_at < lower_bound || game.created_at > upper_bound {
         return Ok(ValidateCallbackResult::Invalid(
             "Game created_at timestamp is too far from action timestamp".to_string()
         ));
     }

    Ok(ValidateCallbackResult::Valid)
}

// Validate updating a Game entry.
// FIX: Accept original_game as argument, remove internal get
pub fn validate_update_game(
    action: &SignedActionHashed, // Action performing the update
    updated_game: Game,          // The proposed new state of the game entry
    original_game: &Game,        // The original state (passed in from validate callback)
) -> ExternResult<ValidateCallbackResult> {

    // --- Use the passed-in original_game ---

    // 2. Check Author: Only players involved can update game status.
    let author = action.action().author();
    if original_game.player_1 != *author && original_game.player_2.as_ref() != Some(author) {
        return Ok(ValidateCallbackResult::Invalid(
            "Only game participants can update the game status".to_string(),
        ));
    }

    // 3. Check Immutability: Ensure critical fields are not changed.
    if updated_game.player_1 != original_game.player_1
        || updated_game.player_2 != original_game.player_2
        || updated_game.created_at != original_game.created_at
    {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot change player_1, player_2, or created_at on game update".to_string(),
        ));
    }

     // 4. PREVENT Real-time State Updates via DHT: Paddle/ball positions should not be updated here.
     if updated_game.player_1_paddle != original_game.player_1_paddle
         || updated_game.player_2_paddle != original_game.player_2_paddle
         || updated_game.ball_x != original_game.ball_x
         || updated_game.ball_y != original_game.ball_y
     {
          // Allow snapshot only if transitioning TO Finished
          if updated_game.game_status == GameStatus::Finished && original_game.game_status != GameStatus::Finished {
               warn!("Allowing update to paddle/ball positions as game transitions to Finished state.");
               // Allow the update in this specific transition case
          } else if updated_game.game_status == GameStatus::Finished && original_game.game_status == GameStatus::Finished {
               // Prevent updates if already finished (even if status isn't changing)
                return Ok(ValidateCallbackResult::Invalid(
                    "Cannot update paddle/ball positions on an already Finished game".to_string(),
                ));
          }
           else {
                // Disallow updates in any other status
                return Ok(ValidateCallbackResult::Invalid(
                    "Cannot update paddle/ball positions via DHT entry update (use signals)".to_string(),
                ));
          }
     }


    // 5. Check Status Transitions: Define allowed state changes.
    match (&original_game.game_status, &updated_game.game_status) {
        (GameStatus::Waiting, GameStatus::InProgress) => {
             if updated_game.player_2.is_none() {
                 return Ok(ValidateCallbackResult::Invalid("Cannot transition to InProgress without Player 2 being set".into()));
             }
             // Note: Further checks like "Is author Player 2?" could be added, but might be overly restrictive
             // depending on how the coordinator handles player joining.
        },
        (GameStatus::InProgress, GameStatus::Finished) => { /* Allow */ },
        (GameStatus::Finished, GameStatus::Finished) => { /* Allow redundant Finished update (checked above for pos changes) */ },
        // Any other transition is invalid
        (from, to) if from == to => { // Allow updating *within* same state ONLY if position snapshot logic above passed (i.e., state is Finished)
             if updated_game.game_status != GameStatus::Finished {
                  return Ok(ValidateCallbackResult::Invalid(format!("No valid updates allowed while game status remains {:?}", from)));
             }
             // If status is Finished and remains Finished, allow (position change already checked)
        },
        (from, to) => {
            return Ok(ValidateCallbackResult::Invalid(format!( "Invalid game status transition from {:?} to {:?}", from, to )));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

// Validate deleting a Game entry.
// Signature matches call from lib.rs where original_game is deserialized first
pub fn validate_delete_game(
    action: &SignedActionHashed, // Action performing the delete
    original_game: Game,         // The game state being deleted
) -> ExternResult<ValidateCallbackResult> {

    // 1. Check Author: Only players involved can delete the game.
    let author = action.action().author();
     if original_game.player_1 != *author && original_game.player_2.as_ref() != Some(author) {
        return Ok(ValidateCallbackResult::Invalid(
            "Only game participants can delete the game".to_string(),
        ));
    }

    // 2. Check Status: Only allow deleting 'Waiting' games
    if original_game.game_status != GameStatus::Waiting {
        return Ok(ValidateCallbackResult::Invalid(
            "Only games in 'Waiting' status can be deleted".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}


// FIX: Remove helper function that uses `get`
// fn must_get_valid_record(action_hash: ActionHash) -> ExternResult<Record> { ... }