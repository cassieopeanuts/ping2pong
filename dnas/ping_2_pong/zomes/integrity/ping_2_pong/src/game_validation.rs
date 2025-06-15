use hdk::prelude::*;
use crate::game::{Game, GameStatus};
use core::time::Duration;
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
pub fn validate_update_game(
    action: &SignedActionHashed,
    updated_game: Game,
    original_game: &Game,
) -> ExternResult<ValidateCallbackResult> {

    let author = action.action().author();

    // --- Author Check ---
    // Allow update if:
    // 1. Author is Player 1
    // 2. Author is the (existing) Player 2
    // 3. Game is Waiting->InProgress AND Author is the NEW Player 2 being added
    let is_player1 = original_game.player_1 == *author;
    let is_existing_player2 = original_game.player_2.as_ref() == Some(author);
    let is_new_player2_joining = original_game.game_status == GameStatus::Waiting
                                 && updated_game.game_status == GameStatus::InProgress
                                 && updated_game.player_2.as_ref() == Some(author);

    if !is_player1 && !is_existing_player2 && !is_new_player2_joining {
         return Ok(ValidateCallbackResult::Invalid(
             "Update author must be Player 1, existing Player 2, or new Player 2 joining a Waiting game".to_string(),
         ));
    }

    // --- Immutability Check ---
    if updated_game.player_1 != original_game.player_1
        || updated_game.created_at != original_game.created_at
        // Allow player_2 to change ONLY when going from Waiting -> InProgress
        || (updated_game.player_2 != original_game.player_2 && !(original_game.game_status == GameStatus::Waiting && updated_game.game_status == GameStatus::InProgress))
    {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot change player_1, created_at, or player_2 (except when joining)".to_string(),
        ));
    }
    // Ensure if player_2 changed, it went from None to Some
     if updated_game.player_2 != original_game.player_2 {
          if original_game.player_2.is_some() || updated_game.player_2.is_none() {
                return Ok(ValidateCallbackResult::Invalid("Player 2 can only be changed from None to Some when joining".into()));
          }
     }


     // --- Prevent Real-time State Updates via DHT ---
     if updated_game.player_1_paddle != original_game.player_1_paddle
         || updated_game.player_2_paddle != original_game.player_2_paddle
         || updated_game.ball_x != original_game.ball_x
         || updated_game.ball_y != original_game.ball_y
     {
          if updated_game.game_status == GameStatus::Finished && original_game.game_status != GameStatus::Finished {
               warn!("Allowing update to paddle/ball positions as game transitions to Finished state.");
          } else if updated_game.game_status == GameStatus::Finished && original_game.game_status == GameStatus::Finished {
                return Ok(ValidateCallbackResult::Invalid( "Cannot update paddle/ball positions on an already Finished game".to_string() ));
          } else {
                return Ok(ValidateCallbackResult::Invalid( "Cannot update paddle/ball positions via DHT entry update (use signals)".to_string() ));
          }
     }

    // --- Status Transitions Check ---
    match (&original_game.game_status, &updated_game.game_status) {
        (GameStatus::Waiting, GameStatus::InProgress) => {
             if updated_game.player_2.is_none() { return Ok(ValidateCallbackResult::Invalid("Cannot transition to InProgress without Player 2 being set".into())); }
        },
        (GameStatus::InProgress, GameStatus::Finished) => { /* Allow */ },
        (GameStatus::Finished, GameStatus::Finished) => { /* Allow */ },
        // Disallow other transitions explicitly for clarity
        (GameStatus::Waiting, GameStatus::Waiting) => return Ok(ValidateCallbackResult::Invalid("No valid updates allowed for 'Waiting' game status".into())),
        (GameStatus::InProgress, GameStatus::InProgress) => return Ok(ValidateCallbackResult::Invalid("No valid updates allowed for 'InProgress' game status".into())),
        (from, to) => { return Ok(ValidateCallbackResult::Invalid(format!( "Invalid game status transition from {:?} to {:?}", from, to ))); }
    }

    Ok(ValidateCallbackResult::Valid)
}

// Validate deleting a Game entry.
pub fn validate_delete_game(
    action: &SignedActionHashed, 
    original_game: Game,         
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
