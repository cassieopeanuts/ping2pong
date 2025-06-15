use hdk::prelude::*;
use crate::statistics::Statistics; 
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

    // Sanity Check Metrics
     if statistics.signal_latency > MAX_LATENCY {
         warn!("Reported signal latency {} exceeds max {}", statistics.signal_latency, MAX_LATENCY);
     }
     if statistics.score_validation_time > MAX_SCORE_VALIDATION_TIME {
          warn!("Reported score_validation_time {} exceeds max {}", statistics.score_validation_time, MAX_SCORE_VALIDATION_TIME);
     }
     if statistics.dht_response_time > MAX_DHT_RESPONSE_TIME {
         warn!("Reported dht_response_time {} exceeds max {}", statistics.dht_response_time, MAX_DHT_RESPONSE_TIME);
     }
    if statistics.network_delay > MAX_NETWORK_DELAY {
         warn!("Reported network_delay {} exceeds max {}", statistics.network_delay, MAX_NETWORK_DELAY);
     }

    // Check Timestamp plausibility
     let action_time = action.action().timestamp();
     let five_minutes = Duration::from_secs(300); 

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
