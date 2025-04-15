// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/statistics_validation.rs
use hdk::prelude::*;
// Remove Game/GameStatus imports as they are no longer checked here
// use crate::{statistics::Statistics, game::{Game, GameStatus}};
use crate::statistics::Statistics; // Keep Statistics import
use core::time::Duration;
use std::ops::{Add, Sub};

// Define maximum allowed values as constants for sanity checks
const MAX_LATENCY: u32 = 30000; // 30 seconds
const MAX_SCORE_VALIDATION_TIME: u32 = 60000; // 60 seconds
const MAX_DHT_RESPONSE_TIME: u32 = 60000; // 60 seconds
const MAX_NETWORK_DELAY: u32 = 30000; // 30 seconds

// Validate creation of a Statistics entry.
pub fn validate_create_statistics(
    action: &SignedActionHashed,
    statistics: Statistics,
) -> ExternResult<ValidateCallbackResult> {
    // 1. REMOVED: Check Game Existence and Status.
    //    This check MUST be performed in the coordinator zome's `create_statistics` function
    //    *before* calling `create_entry`.
    /*
     let maybe_latest_game_record = get_latest_game_record(&statistics.game_id)?; // <-- REMOVED
     // ... rest of game fetching and status check ... <-- REMOVED
    if game.game_status != GameStatus::Finished { ... } // <-- REMOVED
    */

    // 2. REMOVED: Check Author (Participation).
    //    This check also relies on fetching the Game state and belongs in the
    //    coordinator zome's `create_statistics` function.
    /*
    let author = action.action().author();
     if game.player_1 != *author && game.player_2.as_ref() != Some(author) { ... } // <-- REMOVED
    */

    // 3. Sanity Check Metrics: Ensure values are within reasonable bounds.
    //    Keep these checks as they validate the Statistics entry's content.
     if statistics.signal_latency > MAX_LATENCY {
         warn!("Reported signal latency {} exceeds max {}", statistics.signal_latency, MAX_LATENCY);
         // Optionally return Invalid
     }
     if statistics.score_validation_time > MAX_SCORE_VALIDATION_TIME {
          warn!("Reported score_validation_time {} exceeds max {}", statistics.score_validation_time, MAX_SCORE_VALIDATION_TIME);
         // Optionally return Invalid
     }
     if statistics.dht_response_time > MAX_DHT_RESPONSE_TIME {
         warn!("Reported dht_response_time {} exceeds max {}", statistics.dht_response_time, MAX_DHT_RESPONSE_TIME);
        // Optionally return Invalid
     }
    if statistics.network_delay > MAX_NETWORK_DELAY {
         warn!("Reported network_delay {} exceeds max {}", statistics.network_delay, MAX_NETWORK_DELAY);
         // Optionally return Invalid
     }

    // 4. Check Timestamp plausibility
    //    Keep this check.
     let action_time = action.action().timestamp();
     let five_minutes = Duration::from_secs(300); // Using core::time::Duration

     let lower_bound = action_time.sub(five_minutes)
         .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp subtraction error: {}", e))))?;
     let upper_bound = action_time.add(five_minutes)
         .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp addition error: {}", e))))?;

     if statistics.timestamp < lower_bound || statistics.timestamp > upper_bound {
         return Ok(ValidateCallbackResult::Invalid(
             "Statistics timestamp is too far from action timestamp (+/- 5 mins)".to_string()
         ));
     }

    Ok(ValidateCallbackResult::Valid)
}

// REMOVE validate_update_statistics and validate_delete_statistics functions entirely.

// --- REMOVED Helper Function ---
// fn get_latest_game_record(original_game_hash: &ActionHash) -> ExternResult<Option<Record>> { ... } // <-- REMOVED