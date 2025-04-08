// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/player_validation.rs
use hdk::prelude::*;
use crate::player::Player;

// Validate creation of a Player entry.
pub fn validate_create_player(
    action: &SignedActionHashed,
    player: Player,
) -> ExternResult<ValidateCallbackResult> {
    // 1. Check Author: Must match the player_key field.
    if player.player_key != *action.action().author() {
        return Ok(ValidateCallbackResult::Invalid(
            "Player profile can only be created by the player themselves (author must match player_key)".to_string(),
        ));
    }

    // 2. Check Name: Must not be empty and within length limits.
    if player.player_name.trim().is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Player name cannot be empty".to_string()));
    }
    if player.player_name.len() > 50 {
         return Ok(ValidateCallbackResult::Invalid("Player name is too long (max 50 chars)".to_string()));
    }

    // Note: Uniqueness is handled by coordinator before calling create_entry

    Ok(ValidateCallbackResult::Valid)
}

// Validate updating a Player entry.
// FIX: Accept original_player as argument, remove internal get
pub fn validate_update_player(
    action: &SignedActionHashed,
    updated_player: Player,
    original_player: &Player, // The original state (passed in)
) -> ExternResult<ValidateCallbackResult> {
    // --- Use the passed-in original_player instead of fetching ---

    // 2. Check Author: Must be the player themselves.
    if original_player.player_key != *action.action().author() {
        return Ok(ValidateCallbackResult::Invalid(
            "Player profile can only be updated by the player themselves".to_string(),
        ));
    }

    // 3. Check Immutability: player_key cannot change.
    if updated_player.player_key != original_player.player_key {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot change the player_key of a Player profile".to_string(),
        ));
    }

    // 4. Check Name Validity (if changed): Non-empty, length limits.
    if updated_player.player_name != original_player.player_name {
        if updated_player.player_name.trim().is_empty() {
            return Ok(ValidateCallbackResult::Invalid("Updated player name cannot be empty".to_string()));
        }
         if updated_player.player_name.len() > 50 {
             return Ok(ValidateCallbackResult::Invalid("Updated player name is too long (max 50 chars)".to_string()));
         }
         // Note: Uniqueness checks for the new name MUST happen in the coordinator zome
         // before calling update_entry. Integrity zome cannot verify uniqueness across DHT.
         warn!("Player name changed. Uniqueness check relies on coordinator logic and PlayerNameToPlayer link management.");
    }

    Ok(ValidateCallbackResult::Valid)
}

// Validate deleting a Player entry.
// Signature matches call from lib.rs where original_player is deserialized first
pub fn validate_delete_player(
    action: &SignedActionHashed,
    original_player: Player, // Passed directly now
) -> ExternResult<ValidateCallbackResult> {
    // 1. Check Author: Must be the player themselves.
    if original_player.player_key != *action.action().author() {
        return Ok(ValidateCallbackResult::Invalid(
            "Player profile can only be deleted by the player themselves".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

// FIX: Remove helper function that uses `get`
// fn must_get_valid_record(action_hash: ActionHash) -> ExternResult<Record> { ... }