// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/game_validation.rs
use hdk::prelude::*;
use crate::game::{Game, GameStatus};
use std::time::Duration;
use std::ops::{Add, Sub};

// Validate creation of a Game entry.
pub fn validate_create_game(
    action: &SignedActionHashed,
    game: Game,
) -> ExternResult<ValidateCallbackResult> {
    // 1. Check Author: Ensure the creator is Player 1 or Player 2 (if specified).
    let author = action.action().author();
    if game.player_1 != *author && game.player_2.as_ref() != Some(author) {
         // Allow creation if player 2 is set AND player 2 is the author?
         // Coordinator check handles this, but add integrity rule too.
         if !(game.player_2.is_some() && game.player_2.as_ref() == Some(author)) {
            return Ok(ValidateCallbackResult::Invalid(
                "Game creator must be Player 1 (or Player 2 if specified)".to_string(),
            ));
         }
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
pub fn validate_update_game(
    action: &SignedActionHashed, // Action performing the update
    updated_game: Game,          // The proposed new state of the game entry
) -> ExternResult<ValidateCallbackResult> {

    // 1. Get Original Game State: Retrieve the entry state being updated.
    let original_record = must_get_valid_record(action.action().prev_action().ok_or(wasm_error!(
        WasmErrorInner::Guest(
            "Update action must have a prev_action field".to_string()
        )
    ))?.clone())?;
    // <-- FIX: Extract Entry from RecordEntry before TryFrom
    let original_entry = match original_record.entry() {
        RecordEntry::Present(entry) => entry.clone(), // Clone the entry if present
        _ => return Ok(ValidateCallbackResult::Invalid(
            "Original record for game update does not contain a present entry".to_string()
        ))
    };
    let original_game = Game::try_from(original_entry)?; // Now try_from Entry

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
          if updated_game.game_status != GameStatus::Finished { // Allow final state snapshot only when finishing
                return Ok(ValidateCallbackResult::Invalid(
                    "Cannot update paddle/ball positions via DHT entry update (use signals)".to_string(),
                ));
          } else {
              if original_game.game_status == GameStatus::Finished {
                   return Ok(ValidateCallbackResult::Invalid(
                        "Cannot update paddle/ball positions on an already Finished game".to_string(),
                    ));
              }
              warn!("Allowing update to paddle/ball positions as game transitions to Finished state.");
          }
     }


    // 5. Check Status Transitions: Define allowed state changes.
    match (&original_game.game_status, &updated_game.game_status) {
        // Waiting -> InProgress (Player 2 joins or accepts invite)
        (GameStatus::Waiting, GameStatus::InProgress) => {
             if updated_game.player_2.is_none() {
                 return Ok(ValidateCallbackResult::Invalid("Cannot transition to InProgress without Player 2 being set".into()));
             }
        },
        // InProgress -> Finished (Game ends)
        (GameStatus::InProgress, GameStatus::Finished) => { /* Allow */ },
         // Allow Finished -> Finished? (Maybe redundant update?)
         (GameStatus::Finished, GameStatus::Finished) => {
              warn!("Redundant update: Game status is already Finished.");
               if updated_game.player_1_paddle != original_game.player_1_paddle
                 || updated_game.player_2_paddle != original_game.player_2_paddle
                 || updated_game.ball_x != original_game.ball_x
                 || updated_game.ball_y != original_game.ball_y
                {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Cannot update paddle/ball positions on an already Finished game (redundant update)".to_string(),
                    ));
                }
         },
        // Waiting -> Waiting (e.g., updating some metadata if allowed - currently none) - Disallow for now
        (GameStatus::Waiting, GameStatus::Waiting) => return Ok(ValidateCallbackResult::Invalid("No valid updates allowed for 'Waiting' game status".into())),
         // InProgress -> InProgress (e.g., pause/resume? Not defined) - Disallow for now
         (GameStatus::InProgress, GameStatus::InProgress) => return Ok(ValidateCallbackResult::Invalid("No valid updates allowed for 'InProgress' game status".into())),
        // All other transitions are invalid
        (from, to) => {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "Invalid game status transition from {:?} to {:?}", from, to
            )));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

// Validate deleting a Game entry.
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

    // 2. Check Status: Only allow deleting 'Waiting' games (e.g., unaccepted invite, abandoned matchmaking).
    if original_game.game_status != GameStatus::Waiting {
        return Ok(ValidateCallbackResult::Invalid(
            "Only games in 'Waiting' status can be deleted".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}


// Helper to get a record or return Invalid
fn must_get_valid_record(action_hash: ActionHash) -> ExternResult<Record> {
    get(action_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest(format!("Record not found: {}", action_hash))
    ))
}