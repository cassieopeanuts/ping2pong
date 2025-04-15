// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/score_validation.rs
use hdk::prelude::*;
// Remove Game/GameStatus imports as they are no longer checked here
// use crate::{score::Score, game::{Game, GameStatus}};
use crate::score::Score; // Keep Score import
use std::ops::{Add, Sub};


// Validate creation of a Score entry.
pub fn validate_create_score(
    action: &SignedActionHashed,
    score: Score,
) -> ExternResult<ValidateCallbackResult> {
    // 1. REMOVED: Check Game Existence and Status.
    //    This check MUST be performed in the coordinator zome's `create_score` function
    //    *before* calling `create_entry`. The integrity layer cannot safely perform gets.
    /*
    let maybe_latest_game_record = get_latest_game_record(&score.game_id)?; // Use helper <-- REMOVED
    // ... rest of game fetching and status check ... <-- REMOVED
    if game.game_status != GameStatus::Finished { ... } <-- REMOVED
    */

    // 2. REMOVED: Check Player Participation.
    //    This check also relies on fetching the Game state and belongs in the
    //    coordinator zome's `create_score` function.
    /*
    if score.player != game.player_1 && game.player_2.as_ref() != Some(&score.player) { ... } <-- REMOVED
    */

    // 3. REMOVED: Author Check (Optional).
    //    If we wanted to enforce that only the player records their score,
    //    this check is simple and could remain. However, the comment indicated
    //    allowing anyone, so we keep it removed. Coordinator enforces who calls.
    /*
    let author = action.action().author();
    if score.player != *author { ... }
    */

    // 4. Check Score Sanity: Points within reasonable limits.
    //    Keep this check as it only concerns the Score entry itself.
    if score.player_points > 100 {
         warn!("Recorded score {} seems high.", score.player_points);
         // Optionally return Invalid if a hard limit is desired in integrity:
         // return Ok(ValidateCallbackResult::Invalid("Score points seem unreasonably high (> 100)".to_string()));
    }

    // 5. Check Timestamp plausibility
    //    Keep this check - compares action timestamp with entry timestamp.
     let action_time = action.action().timestamp();
     let five_minutes_duration = core::time::Duration::from_secs(300); // Using core::time::Duration

     let lower_bound = action_time.sub(five_minutes_duration)
         .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp subtraction error: {}", e))))?;
     let upper_bound = action_time.add(five_minutes_duration)
          .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp addition error: {}", e))))?;

     if score.created_at < lower_bound || score.created_at > upper_bound {
         return Ok(ValidateCallbackResult::Invalid(
             "Score created_at timestamp is too far from action timestamp (+/- 5 mins)".to_string()
         ));
     }

    Ok(ValidateCallbackResult::Valid)
}

// REMOVE validate_update_score and validate_delete_score functions entirely
// as Scores are intended to be immutable after creation in this design.

// --- REMOVED Helper Function ---
// fn get_latest_game_record(original_game_hash: &ActionHash) -> ExternResult<Option<Record>> { ... } // <-- REMOVED

