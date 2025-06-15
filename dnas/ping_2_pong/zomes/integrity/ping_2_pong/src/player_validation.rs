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

    Ok(ValidateCallbackResult::Valid)
}

// Validate updating a Player entry.
pub fn validate_update_player(
    action: &SignedActionHashed,
    updated_player: Player,
    original_player: &Player, 
) -> ExternResult<ValidateCallbackResult> {

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
         warn!("Player name changed. Uniqueness check relies on coordinator logic and PlayerNameToPlayer link management.");
    }

    Ok(ValidateCallbackResult::Valid)
}

// Validate deleting a Player entry.
pub fn validate_delete_player(
    action: &SignedActionHashed,
    original_player: Player, 
) -> ExternResult<ValidateCallbackResult> {
    // 1. Check Author: Must be the player themselves.
    if original_player.player_key != *action.action().author() {
        return Ok(ValidateCallbackResult::Invalid(
            "Player profile can only be deleted by the player themselves".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
