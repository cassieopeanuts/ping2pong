use hdi::prelude::*;
use hdk::prelude::*;

use log::info;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum GameStatus {
    Waiting,
    InProgress,
    Finished,
}


#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct Game {
    pub player_1: AgentPubKey,
    pub player_2: Option<AgentPubKey>,
    pub created_at: Timestamp,
    pub game_status: GameStatus,
    pub player_1_paddle: u32,
    pub player_2_paddle: u32,
    pub ball_x: u32,
    pub ball_y: u32,

}

// Validate_create_game function
pub fn validate_create_game(
    action: EntryCreationAction,
) -> ExternResult<ValidateCallbackResult> {
    info!(
        "Validating creation of game by agent: {:?}",

        action.author()
    );

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_game(
    action: Update,
    updated_game: Game,
    _original_action: EntryCreationAction,
    original_game: Game,
) -> ExternResult<ValidateCallbackResult> {
    // Only player_1 or player_2 can update the game
    if action.author != original_game.player_1
    && match &original_game.player_2 {
        Some(player2) => action.author != *player2,
        None => true,
    }
{
    return Ok(ValidateCallbackResult::Invalid(
        "Only game participants can update the game".into(),
    ));
}


    // Validate game status transitions
    match (&original_game.game_status, &updated_game.game_status) {
        (GameStatus::Waiting, GameStatus::InProgress) => (),
        (GameStatus::InProgress, GameStatus::Finished) => (),
        _ => {
            return Ok(ValidateCallbackResult::Invalid(
                "Invalid game status transition".into(),
            ))
        }
    }

    // Prevent reverting to a previous state
    if let GameStatus::Waiting = updated_game.game_status {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot revert game status to 'Waiting'".into(),
        ));
    }

    // Ensure player_1 and player_2 are not changed
    if original_game.player_1 != updated_game.player_1 || original_game.player_2 != updated_game.player_2 {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot change players of an ongoing game".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_game(
    action: Delete,
    _original_action: EntryCreationAction,
    original_game: Game,
) -> ExternResult<ValidateCallbackResult> {
    // Only player_1 or player_2 can delete the game
    if action.author != original_game.player_1
    && match &original_game.player_2 {
        Some(player2) => action.author != *player2,
        None => true,
    }
{
    return Ok(ValidateCallbackResult::Invalid(
        "Only game participants can delete the game".into(),
    ));
}


    // Only allow deletion if the game is not in progress
    match original_game.game_status {
        GameStatus::Waiting => (),
        _ => {
            return Ok(ValidateCallbackResult::Invalid(
                "Can only delete games that are in 'Waiting' status".into(),
            ))
        }
    }

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_player_1_to_games(
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
    let _game: crate::Game = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_player_1_to_games(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_player_2_to_games(
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
    let _game: crate::Game = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_player_2_to_games(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_game_updates(
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
    let _game: crate::Game = record
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
    let _game: crate::Game = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_game_updates(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // Prevent deletion of GameUpdates links
    Ok(ValidateCallbackResult::Invalid(
        "GameUpdates links cannot be deleted".to_string(),
    ))
}

// Stub for validating creation of a GameIdToGame link.
pub fn validate_create_link_game_id_to_game(
    _action: CreateLink,
    _base_address: AnyLinkableHash,
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: Implement detailed validation.
    Ok(ValidateCallbackResult::Valid)
}

// Stub for validating deletion of a GameIdToGame link.
pub fn validate_delete_link_game_id_to_game(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: Implement detailed validation.
    Ok(ValidateCallbackResult::Valid)
}
