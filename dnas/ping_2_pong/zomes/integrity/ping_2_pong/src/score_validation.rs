use hdk::prelude::*;
use crate::score::Score; 
use std::ops::{Add, Sub};


// Validate creation of a Score entry.
pub fn validate_create_score(
    action: &SignedActionHashed,
    score: Score,
) -> ExternResult<ValidateCallbackResult> {
    // Check Score Sanity
    if score.player_points > 10 {
         warn!("Recorded score {} seems high.", score.player_points);
    }

    // Check Timestamp plausibility
     let action_time = action.action().timestamp();
     let five_minutes_duration = core::time::Duration::from_secs(300); 

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
