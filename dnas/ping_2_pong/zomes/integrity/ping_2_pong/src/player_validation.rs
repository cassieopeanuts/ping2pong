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

    // 2. Check Name: Must not be empty.
    // Uniqueness is checked via PlayerNameToPlayer links and coordinator logic,
    // but basic non-empty check here.
    if player.player_name.trim().is_empty() {
        return Ok(ValidateCallbackResult::Invalid("Player name cannot be empty".to_string()));
    }
    // Max length check?
    if player.player_name.len() > 50 {
         return Ok(ValidateCallbackResult::Invalid("Player name is too long (max 50 chars)".to_string()));
    }


    // 3. Check Agent Existence: Ensure the agent key is valid (usually implicitly true if they authored).
    // let agent_info = must_get_agent_info(player.player_key.clone())?; // Could add this check

    Ok(ValidateCallbackResult::Valid)
}

// Validate updating a Player entry.
pub fn validate_update_player(
    action: &SignedActionHashed,
    updated_player: Player,
) -> ExternResult<ValidateCallbackResult> {
     // 1. Get Original Player State
     let original_record = must_get_valid_record(action.action().prev_action().ok_or(wasm_error!(
        WasmErrorInner::Guest("Update action must have a prev_action field".to_string())
    ))?.clone())?;
    
     // <-- FIX: Extract Entry from RecordEntry before TryFrom
    let original_entry = match original_record.entry() {
        RecordEntry::Present(entry) => entry.clone(), // Clone the entry if present
        _ => return Ok(ValidateCallbackResult::Invalid(
            "Original record for player update does not contain a present entry".to_string()
        ))
    };
    let original_player = Player::try_from(original_entry)?; // Now try_from Entry


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
    // Integrity rules CANNOT easily enforce uniqueness on update because it requires
    // checking for *other* players' links/names. Coordinator must handle uniqueness checks
    // and potentially manage updating the PlayerNameToPlayer link if name changes are allowed.
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

    // TODO: Consider implications. Should deleting profile prevent future play? Or just remove name?
    // For now, allow deletion by the owner.

    Ok(ValidateCallbackResult::Valid)
}

// Helper (takes ActionHash by value now)
fn must_get_valid_record(action_hash: ActionHash) -> ExternResult<Record> {
     get(action_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest(format!("Record not found: {}", action_hash))
    ))
}