// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/statistics_validation.rs
use hdk::prelude::*;
use crate::{statistics::Statistics, game::{Game, GameStatus}}; // Import Game for validation
// Add imports for timestamp arithmetic if not already present globally
use core::time::Duration;
use std::ops::{Add, Sub};

// Define maximum allowed values as constants for sanity checks
const MAX_LATENCY: u32 = 30000; // 30 seconds
// Adjust max value based on potential new meaning or remove check
const MAX_SCORE_VALIDATION_TIME: u32 = 60000; // 60 seconds
const MAX_DHT_RESPONSE_TIME: u32 = 60000; // 60 seconds
const MAX_NETWORK_DELAY: u32 = 30000; // 30 seconds

// Validate creation of a Statistics entry.
pub fn validate_create_statistics(
    action: &SignedActionHashed,
    statistics: Statistics,
) -> ExternResult<ValidateCallbackResult> {
    // 1. Check Game Existence and Status: Fetch the latest game state.
     let maybe_latest_game_record = get_latest_game_record(&statistics.game_id)?;
    let game_record = match maybe_latest_game_record {
        Some(record) => record,
        None => return Ok(ValidateCallbackResult::Invalid(format!(
            "Referenced game ({}) does not exist or has no latest state", statistics.game_id
        ))),
    };
    // <-- FIX: Extract Entry from RecordEntry before TryFrom
    let game_entry = match game_record.entry() {
        RecordEntry::Present(entry) => entry.clone(),
        _ => return Ok(ValidateCallbackResult::Invalid(
            "Latest game record for statistics validation does not contain a present entry".to_string()
        ))
    };
    let game = Game::try_from(game_entry)?; // Now try_from Entry


    // Ensure game status is Finished.
    if game.game_status != GameStatus::Finished {
        return Ok(ValidateCallbackResult::Invalid(
            "Statistics can only be recorded for games with 'Finished' status".to_string(),
        ));
    }

    // 2. Check Author: Should be one of the game participants.
    let author = action.action().author();
     if game.player_1 != *author && game.player_2.as_ref() != Some(author) {
        return Ok(ValidateCallbackResult::Invalid(
            "Only game participants can record statistics".to_string(),
        ));
    }

    // 3. Sanity Check Metrics: Ensure values are within reasonable bounds.
     if statistics.signal_latency > MAX_LATENCY {
         warn!("Reported signal latency {} exceeds max {}", statistics.signal_latency, MAX_LATENCY);
         // return Ok(ValidateCallbackResult::Invalid("Signal latency value is unreasonably high".into()));
     }
     if statistics.score_validation_time > MAX_SCORE_VALIDATION_TIME {
          warn!("Reported score_validation_time {} exceeds max {}", statistics.score_validation_time, MAX_SCORE_VALIDATION_TIME);
         // return Ok(ValidateCallbackResult::Invalid("Score validation time value is unreasonably high".into()));
     }
     if statistics.dht_response_time > MAX_DHT_RESPONSE_TIME {
         warn!("Reported dht_response_time {} exceeds max {}", statistics.dht_response_time, MAX_DHT_RESPONSE_TIME);
        // return Ok(ValidateCallbackResult::Invalid("DHT response time value is unreasonably high".into()));
     }
    if statistics.network_delay > MAX_NETWORK_DELAY {
         warn!("Reported network_delay {} exceeds max {}", statistics.network_delay, MAX_NETWORK_DELAY);
         // return Ok(ValidateCallbackResult::Invalid("Network delay value is unreasonably high".into()));
     }

    // 4. Check Timestamp plausibility
     let action_time = action.action().timestamp();
     let five_minutes = Duration::from_secs(300); // Using core::time::Duration

     // <-- FIX: Use .map_err() before '?' for timestamp arithmetic
     let lower_bound = action_time.sub(five_minutes)
         .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp subtraction error: {}", e))))?;
     let upper_bound = action_time.add(five_minutes)
         .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp addition error: {}", e))))?;

     if statistics.timestamp < lower_bound || statistics.timestamp > upper_bound {
         return Ok(ValidateCallbackResult::Invalid(
             "Statistics timestamp is too far from action timestamp".to_string()
         ));
     }

    Ok(ValidateCallbackResult::Valid)
}

// REMOVE validate_update_statistics and validate_delete_statistics functions entirely.

// --- Helper Functions --- (Copied from score_validation, ensure import LinkTypes)
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