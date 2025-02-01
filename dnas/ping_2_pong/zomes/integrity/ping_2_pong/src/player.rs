use hdi::prelude::*;
use hdk::prelude::{
    get, get_links, GetLinksInputBuilder, GetOptions,
};
use crate::{Game, GameStatus, LinkTypes};
use crate::anchor_for;

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct Player {
    pub player_key: AgentPubKey,
    pub player_name: String,
}

// Helper function to check if a player name is unique
pub fn is_player_name_unique(player_name: &str) -> ExternResult<bool> {
    // Use an anchor for all players rather than ().
    let base = anchor_for("players")?;
    let player_links = get_links(
        GetLinksInputBuilder::try_new(base, LinkTypes::PlayerToPlayers)?.build(),
    )?;
    
    for link in player_links {
        let player_hash = link.target.into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid player hash".to_string())),
        )?;
        let player_record = get(player_hash, GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Player record not found".to_string()
            )))?;
        // IMPORTANT: Convert to Player (not Game)
        if let Some(existing_player) = player_record
            .entry()
            .to_app_option::<Player>()
            .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        {
            if existing_player.player_name.to_lowercase() == player_name.to_lowercase() {
                return Ok(false);
            }
        }
    }
    
    Ok(true)
}

pub fn validate_create_player(
    action: EntryCreationAction,
    player: Player,
) -> ExternResult<ValidateCallbackResult> {
    // Ensure the agent creating the player matches the player_key
    if action.author().clone() != player.player_key {
        return Ok(ValidateCallbackResult::Invalid(
            "Player can only be created by themselves".into(),
        ));
    }

    // Ensure player_name is unique
    if !is_player_name_unique(&player.player_name)? {
        return Ok(ValidateCallbackResult::Invalid(
            "Player name must be unique".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_player(
    action: Update,
    updated_player: Player,
    _original_action: EntryCreationAction,
    original_player: Player,
) -> ExternResult<ValidateCallbackResult> {
    // Players can only update their own profiles.
    if action.author != original_player.player_key {
        return Ok(ValidateCallbackResult::Invalid(
            "Players can only update their own profiles".into(),
        ));
    }

    // If player_name is being updated, ensure the new name is unique.
    if updated_player.player_name.to_lowercase() != original_player.player_name.to_lowercase() {
        if !is_player_name_unique(&updated_player.player_name)? {
            return Ok(ValidateCallbackResult::Invalid(
                "Player name must be unique".into(),
            ));
        }
        // Optionally, you could update a link here—but if it's not needed,
        // you can simply skip any link update logic.
    }

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_player(
    action: Delete,
    _original_action: EntryCreationAction,
    original_player: Player,
) -> ExternResult<ValidateCallbackResult> {
    // Only the player themselves can delete their profile
    if action.author != original_player.player_key {
        return Ok(ValidateCallbackResult::Invalid(
            "Players can only delete their own profiles".into(),
        ));
    }

    // Ensure the player is not part of any ongoing games
    for link_type in &[LinkTypes::Player1ToGames, LinkTypes::Player2ToGames] {
        let player_games = get_links(
            GetLinksInputBuilder::try_new(original_player.player_key.clone(), *link_type)?.build(),
        )?;
        for link in player_games {
            let game_hash = link.target.into_action_hash().ok_or(
                wasm_error!(WasmErrorInner::Guest("Invalid game hash".to_string())),
            )?;
            let game_record = get(game_hash, GetOptions::default())?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Game record not found".to_string()
                )))?;
            if let Some(game) = game_record
                .entry()
                .to_app_option::<Game>()
                .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
            {
                if game.game_status == GameStatus::InProgress {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Cannot delete player who is in an ongoing game".into(),
                    ));
                }
            }
        }
    }

    // Optionally, clean up associated scores and statistics here or restrict deletion based on dependencies

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_player_to_players(
    _action: CreateLink,
    _base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    let player_hash = target_address
        .into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No action hash associated with link".to_string()
        )))?;
    let player_record = get(player_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Player record not found".into())))?;
    let _player: Player = player_record
        .entry()
        .to_app_option::<Player>()
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_player_to_players(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_player_updates(
    _action: CreateLink,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    let base_hash = base_address
        .into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No action hash associated with link (base)".to_string()
        )))?;
    let base_record = must_get_valid_record(base_hash)?;
    let _player: Player = base_record
        .entry()
        .to_app_option::<Player>()
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry (base)".to_string()
        )))?;
    let target_hash = target_address
        .into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No action hash associated with link (target)".to_string()
        )))?;
    let target_record = must_get_valid_record(target_hash)?;
    let _player: Player = target_record
        .entry()
        .to_app_option::<Player>()
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry (target)".to_string()
        )))?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_player_updates(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "PlayerUpdates links cannot be deleted".to_string(),
    ))
}

pub fn validate_create_link_player_name_to_player(
    _action: CreateLink,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // Ensure the target_address references a Player entry.
    let player_hash = target_address
        .into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No action hash associated with link (target)".to_string()
        )))?;
    let player_record = get(player_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Player record not found".into())))?;
    let player: Player = player_record
        .entry()
        .to_app_option::<Player>()
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid Player entry".into())))?;

    // Convert the base_address to a string.
    // Since base_address is an AnyLinkableHash, we can convert its raw bytes to a Vec<u8>
    // and then attempt to interpret that as UTF-8.
    let player_name = String::from_utf8(base_address.as_ref().to_vec()).map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(
            "PlayerNameToPlayer link base address must be valid UTF-8".into()
        ))
    })?;
    

    // Verify that the player_name from the link matches the player's stored name.
    if player.player_name.to_lowercase() != player_name.to_lowercase() {
        return Ok(ValidateCallbackResult::Invalid(
            "Player name in link does not match Player entry".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

// Stub for validating deletion of a PlayerNameToPlayer link.
pub fn validate_delete_link_player_name_to_player(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // For example, you might want to prevent deletion:
    Ok(ValidateCallbackResult::Invalid(
        "PlayerNameToPlayer links cannot be deleted".into(),
    ))
}
