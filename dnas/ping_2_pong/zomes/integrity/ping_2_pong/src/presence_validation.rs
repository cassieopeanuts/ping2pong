// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/presence_validation.rs
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
     let action_time_ms = action.action().timestamp().as_millis();
     let five_minutes_ms = 300_000; // 5 * 60 * 1000
     if presence.timestamp < action_time_ms.saturating_sub(five_minutes_ms).try_into().unwrap() || presence.timestamp > action_time_ms.saturating_add(five_minutes_ms).try_into().unwrap() {
         return Ok(ValidateCallbackResult::Invalid(
             "Presence timestamp is too far from action timestamp (+/- 5 minutes)".to_string()
         ));
     }

    Ok(ValidateCallbackResult::Valid)
}

// No updates or deletes typically needed for simple presence markers.
// If needed, add validate_update_presence / validate_delete_presence.