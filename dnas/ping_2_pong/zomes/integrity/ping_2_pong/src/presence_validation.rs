use hdk::prelude::*;
use crate::presence::Presence;

pub fn validate_create_presence(
    action: &SignedActionHashed,
    presence: Presence,
) -> ExternResult<ValidateCallbackResult> {
    // 1. Check Author matches agent_pubkey
    if presence.agent_pubkey != *action.action().author() {
        return Ok(ValidateCallbackResult::Invalid(
            "Presence entry author must match agent_pubkey field".to_string(),
        ));
    }

    // 2. Check Timestamp plausibility (not too far in past/future)
     let action_time_ms = action.action().timestamp().as_millis(); // This is i64
     let five_minutes_ms: i64 = 300_000; // 5 * 60 * 1000

     // Calculate bounds as i64
     let lower_bound_i64 = action_time_ms.saturating_sub(five_minutes_ms);
     let upper_bound_i64 = action_time_ms.saturating_add(five_minutes_ms);

     // Cast bounds to u64. This is safe assuming realistic (non-negative) timestamps
     // and that saturating_sub doesn't produce negative results we care about here.
     if presence.timestamp < (lower_bound_i64 as u64) || presence.timestamp > (upper_bound_i64 as u64) {
         if lower_bound_i64 < 0 {
              warn!("Calculated lower bound for presence timestamp was negative: {}", lower_bound_i64);
         }
         // Return invalid only if the u64 comparison fails
         if presence.timestamp < (lower_bound_i64 as u64) || presence.timestamp > (upper_bound_i64 as u64) {
            return Ok(ValidateCallbackResult::Invalid(
                "Presence timestamp is too far from action timestamp (+/- 5 minutes)".to_string()
            ));
         }
     }

    Ok(ValidateCallbackResult::Valid)
}

