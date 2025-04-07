// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/score_validation.rs
use hdk::prelude::*;
use crate::{score::Score, game::{Game, GameStatus}}; // Import Game for validation
use std::ops::{Add, Sub};


// Validate creation of a Score entry.
pub fn validate_create_score(
    action: &SignedActionHashed,
    score: Score,
) -> ExternResult<ValidateCallbackResult> {
    // 1. Check Game Existence and Status: Fetch the game this score refers to.
    // NOTE: score.game_id should be the ORIGINAL ActionHash of the game.
    // We need the LATEST state of the game to check its status.
    let maybe_latest_game_record = get_latest_game_record(&score.game_id)?; // Use helper
    let game_record = match maybe_latest_game_record {
        Some(record) => record,
        None => return Ok(ValidateCallbackResult::Invalid(format!(
            "Referenced game ({}) does not exist or has no latest state", score.game_id
        ))),
    };
    // <-- FIX: Extract Entry from RecordEntry before TryFrom
    let game_entry = match game_record.entry() {
        RecordEntry::Present(entry) => entry.clone(),
        _ => return Ok(ValidateCallbackResult::Invalid(
            "Latest game record for score validation does not contain a present entry".to_string()
        ))
    };
    let game = Game::try_from(game_entry)?; // Now try_from Entry

    // Ensure game status is Finished.
    if game.game_status != GameStatus::Finished {
        return Ok(ValidateCallbackResult::Invalid(
            "Scores can only be recorded for games with 'Finished' status".to_string(),
        ));
    }

    // 2. Check Player Participation: Ensure score.player was in the game.
     if score.player != game.player_1 && game.player_2.as_ref() != Some(&score.player) {
        return Ok(ValidateCallbackResult::Invalid(
            "Score player did not participate in the referenced game".to_string(),
        ));
    }

    // 3. Check Author? Should only players record scores? Or anyone?
    // Let's allow anyone for now, assuming coordinator ensures correctness.
    // let author = action.action().author();
    // if score.player != *author { // Only allow player to record their own score?
    //     return Ok(ValidateCallbackResult::Invalid("Score can only be created by the player it belongs to".to_string()));
    // }

    // 4. Check Score Sanity: Points within reasonable limits (e.g., <= 100?)
    // Game logic enforces 10 points, but add a sanity check here.
    if score.player_points > 100 {
         warn!("Recorded score {} seems high.", score.player_points);
         // return Ok(ValidateCallbackResult::Invalid("Score points seem unreasonably high (> 100)".to_string()));
    }

    // 5. Check Timestamp plausibility
     let action_time = action.action().timestamp();
     // Note: Using std::time::Duration requires bringing that crate in potentially.
     // hdk::prelude::Duration might be available depending on HDK version.
     // Let's assume hdk::prelude::Duration works for simplicity. If not, use i64 directly.
     let five_minutes_duration = core::time::Duration::from_secs(300); // Using core::time::Duration

     // <-- FIX: Use .map_err() before '?' for timestamp arithmetic
     let lower_bound = action_time.sub(five_minutes_duration)
         .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp subtraction error: {}", e))))?;
     let upper_bound = action_time.add(five_minutes_duration)
          .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp addition error: {}", e))))?;

     if score.created_at < lower_bound || score.created_at > upper_bound {
         return Ok(ValidateCallbackResult::Invalid(
             "Score created_at timestamp is too far from action timestamp".to_string()
         ));
     }

    Ok(ValidateCallbackResult::Valid)
}

// REMOVE validate_update_score and validate_delete_score functions entirely
// as Scores are immutable after creation.

// --- Helper Functions ---

// Gets the latest record for a game given the original action hash
fn get_latest_game_record(original_game_hash: &ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(original_game_hash.clone(), crate::LinkTypes::GameUpdates)?.build(),
    )?;
    let latest_link = links
        .into_iter()
        .max_by(|link_a, link_b| link_a.timestamp.cmp(&link_b.timestamp));

    let latest_game_hash = match latest_link {
        Some(link) => {
            link.target
                .into_action_hash()
                .ok_or(wasm_error!(WasmErrorInner::Guest("GameUpdates link target is not an ActionHash".to_string())))?
        }
        None => original_game_hash.clone(), // No updates, use original hash
    };
    get(latest_game_hash, GetOptions::default())
}