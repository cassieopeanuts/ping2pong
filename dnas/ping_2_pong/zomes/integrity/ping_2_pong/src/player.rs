use hdi::prelude::*;

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct Player {
    pub player_key: AgentPubKey,
    pub player_name: String,
}

// Helper function to check if a player name is unique
fn is_player_name_unique(player_name: &str) -> ExternResult<bool> {
    // Retrieve all players
    let player_links = get_links(
        GetLinksInputBuilder::try_new((), LinkTypes::PlayerToPlayers)?.build(),
    )?;
    
    for link in player_links {
        let player_hash = link.target.into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid player hash".to_string())),
        )?;
        let player_record = get(player_hash, GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Player record not found".to_string()
            )))?;
        if let Some(existing_player) = player_record
            .entry()
            .to_app_option::<Player>()?
        {
            if existing_player.player_name.to_lowercase() == player_name.to_lowercase() {
                return Ok(false);
            }
        }
    }
    
    Ok(true)
}

// Updated validate_create_player function
pub fn validate_create_player(
    action: EntryCreationAction,
    player: Player,
) -> ExternResult<ValidateCallbackResult> {
    // Ensure the agent creating the player matches the player_key
    if action.author != player.player_key {
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
    original_action: EntryCreationAction,
    original_player: Player,
) -> ExternResult<ValidateCallbackResult> {
    // Players can only update their own profiles
    if action.author != original_player.player_key {
        return Ok(ValidateCallbackResult::Invalid(
            "Players can only update their own profiles".into(),
        ));
    }

    // If player_name is being updated, ensure the new name is unique
    if updated_player.player_name.to_lowercase() != original_player.player_name.to_lowercase() {
        if !is_player_name_unique(&updated_player.player_name)? {
            return Ok(ValidateCallbackResult::Invalid(
                "Player name must be unique".into(),
            ));
        }

        // Update the PlayerNameToPlayer link
        // Remove the old link
        let old_link = get_links(
            GetLinksInputBuilder::try_new(original_player.player_name.to_lowercase().into(), LinkTypes::PlayerNameToPlayer)?.build(),
        )?;
        for link in old_link {
            delete_link(link.create_link_hash)?;
        }

        // Create a new link with the updated name
        create_link(
            updated_player.player_name.to_lowercase().into(),
            action.entry_address.clone(),
            LinkTypes::PlayerNameToPlayer,
            (),
        )?;
    }

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_player(
    action: Delete,
    original_action: EntryCreationAction,
    original_player: Player,
) -> ExternResult<ValidateCallbackResult> {
    // Only the player themselves can delete their profile
    if action.author != original_player.player_key {
        return Ok(ValidateCallbackResult::Invalid(
            "Players can only delete their own profiles".into(),
        ));
    }

    // Ensure the player is not part of any ongoing games
    let player_games_1 = get_links(
        GetLinksInputBuilder::try_new(original_player.player_key.clone(), LinkTypes::Player1ToGames)?.build(),
    )?;
    for link in player_games_1 {
        let game_hash = link.target.into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid game hash".to_string())),
        )?;
        let game_record = get(game_hash, GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Game record not found".to_string()
            )))?;
        if let Some(game) = game_record
            .entry()
            .to_app_option::<Game>()?
        {
            if game.game_status == GameStatus::InProgress {
                return Ok(ValidateCallbackResult::Invalid(
                    "Cannot delete player who is in an ongoing game".into(),
                ));
            }
        }
    }

    let player_games_2 = get_links(
        GetLinksInputBuilder::try_new(original_player.player_key.clone(), LinkTypes::Player2ToGames)?.build(),
    )?;
    for link in player_games_2 {
        let game_hash = link.target.into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid game hash".to_string())),
        )?;
        let game_record = get(game_hash, GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Game record not found".to_string()
            )))?;
        if let Some(game) = game_record
            .entry()
            .to_app_option::<Game>()?
        {
            if game.game_status == GameStatus::InProgress {
                return Ok(ValidateCallbackResult::Invalid(
                    "Cannot delete player who is in an ongoing game".into(),
                ));
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
    let action_hash =
        target_address
            .into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "No action hash associated with link".to_string()
            )))?;
    let record = must_get_valid_record(action_hash)?;
    let _player: crate::Player = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
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
    let action_hash = base_address
        .into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No action hash associated with link".to_string()
        )))?;
    let record = must_get_valid_record(action_hash)?;
    let _player: crate::Player = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    let action_hash =
        target_address
            .into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "No action hash associated with link".to_string()
            )))?;
    let record = must_get_valid_record(action_hash)?;
    let _player: crate::Player = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
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
    _base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // Ensure the target_address references a Player entry
    let player_hash = target_address
        .into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No action hash associated with link".to_string()
        )))?;
    let player_record = get(player_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Player record not found".into())))?;
    let player = player_record
        .entry()
        .to_app_option::<Player>()?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid Player entry".into())))?;

    // Ensure the base_address corresponds to the player_name
    let player_name = match base_address.into_agent_pub_key() {
        Ok(_) => {
            return Ok(ValidateCallbackResult::Invalid(
                "Base address for PlayerNameToPlayer link should be the player_name string, not an AgentPubKey".into(),
            ));
        }
        Err(_) => {
            // Assuming the base_address is a string representing player_name
            String::from_utf8(base_address.to_owned()).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest(
                    "PlayerNameToPlayer link base address must be a valid UTF-8 string".into()
                ))
            })?
        }
    };

    // Optionally, verify that the player_name in the link matches the player's name
    if player.player_name.to_lowercase() != player_name.to_lowercase() {
        return Ok(ValidateCallbackResult::Invalid(
            "Player name in link does not match Player entry".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}
