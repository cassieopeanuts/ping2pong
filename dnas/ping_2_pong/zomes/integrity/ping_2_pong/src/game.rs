use hdi::prelude::*;

#[macro_use]
extern crate log;

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
    pub game_id: String,
    pub player_1: AgentPubKey,
    pub player_2: AgentPubKey,
    pub created_at: Timestamp,
    pub game_status: GameStatus,
}

// Helper function to get game hash
fn get_game_hash_by_id(game_id: &str) -> ExternResult<Option<ActionHash>> {
    // Fetch links from game_id to Game entries
    let links = get_links(
        GetLinksInputBuilder::try_new(game_id.to_lowercase().into(), LinkTypes::GameIdToGame)?.build(),
    )?;

    // If a link exists, return the ActionHash
    if let Some(link) = links.first() {
        let game_hash = link.target.into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid game hash".to_string())),
        )?;
        Ok(Some(game_hash))
    } else {
        Ok(None)
    }
}

// Helper function to check if a player exists
fn player_exists(agent_pub_key: &AgentPubKey) -> ExternResult<bool> {
    let links = get_links(
        GetLinksInputBuilder::try_new(*agent_pub_key, LinkTypes::PlayerToPlayers)?.build(),
    )?;
    Ok(!links.is_empty())
}

// Helper function to chexk if player is already in an ongoing game
fn is_player_in_ongoing_game(player_pub_key: &AgentPubKey) -> ExternResult<bool> {
    // Fetch all games where the player is player_1
    let player1_games = get_links(
        GetLinksInputBuilder::try_new(*player_pub_key, LinkTypes::Player1ToGames)?.build(),
    )?;
    
    for link in player1_games {
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
                return Ok(true);
            }
        }
    }

    // Fetch all games where the player is player_2
    let player2_games = get_links(
        GetLinksInputBuilder::try_new(*player_pub_key, LinkTypes::Player2ToGames)?.build(),
    )?;
    
    for link in player2_games {
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
                return Ok(true);
            }
        }
    }

    Ok(false)
}

// Validate_create_game function
pub fn validate_create_game(
    action: EntryCreationAction,
    game: Game,
) -> ExternResult<ValidateCallbackResult> {
        // Log the creation attempt
        info!(
            "Validating creation of game with ID: {} by agent: {:?}",
            game.game_id, action.author
        );

    // Ensure unique game_id
    let existing_games = get_links(
        GetLinksInputBuilder::try_new((), LinkTypes::GameUpdates)?.build(),
    )?;
    for link in existing_games {
        let existing_game_hash = link.target.into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid game hash".to_string())),
        )?;
        let existing_game_record = get(existing_game_hash, GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Game record not found".to_string()
            )))?;
        if let Some(existing_game) = existing_game_record
            .entry()
            .to_app_option::<Game>()?
        {
            if existing_game.game_id == game.game_id {
                return Ok(ValidateCallbackResult::Invalid(
                    "Game ID must be unique".into(),
                ));
            }
        }
    }

    // Ensure player_1 and player_2 are valid registered players
    if !player_exists(&game.player_1)? {
        return Ok(ValidateCallbackResult::Invalid(
            "Player 1 is not a registered player".into(),
        ));
    }

    if !player_exists(&game.player_2)? {
        return Ok(ValidateCallbackResult::Invalid(
            "Player 2 is not a registered player".into(),
        ));
    }

    // Ensure player_1 and player_2 are not the same
    if game.player_1 == game.player_2 {
        return Ok(ValidateCallbackResult::Invalid(
            "Player 1 and Player 2 cannot be the same agent".into(),
        ));
    }

    // Ensure game_status is initially Waiting
    if game.game_status != GameStatus::Waiting {
        return Ok(ValidateCallbackResult::Invalid(
            "Newly created games must have status 'Waiting'".into(),
        ));
    }


    // Ensure player_1 is not in another ongoing game
    if is_player_in_ongoing_game(&game.player_1)? {
        return Ok(ValidateCallbackResult::Invalid(
            "Player 1 is already in an ongoing game".into(),
        ));
    }

    // Ensure player_2 is not in another ongoing game
    if is_player_in_ongoing_game(&game.player_2)? {
        return Ok(ValidateCallbackResult::Invalid(
            "Player 2 is already in an ongoing game".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_game(
    action: Update,
    updated_game: Game,
    original_action: EntryCreationAction,
    original_game: Game,
) -> ExternResult<ValidateCallbackResult> {
    // Only player_1 or player_2 can update the game
    if action.author != original_game.player_1 && action.author != original_game.player_2 {
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
    original_action: EntryCreationAction,
    original_game: Game,
) -> ExternResult<ValidateCallbackResult> {
    // Only player_1 or player_2 can delete the game
    if action.author != original_game.player_1 && action.author != original_game.player_2 {
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