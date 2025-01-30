use hdi::prelude::*;

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct Score {
    pub game_id: String,
    pub player: AgentPubKey,
    pub player_points: u32,
}

pub fn validate_create_score(
    action: EntryCreationAction,
    score: Score,
) -> ExternResult<ValidateCallbackResult> {
    // Ensure the game_id exists and retrieve its ActionHash
    let game_hash = match get_game_hash_by_id(&score.game_id)? {
        Some(hash) => hash,
        None => {
            return Ok(ValidateCallbackResult::Invalid(
                "Game ID does not exist".into(),
            ));
        }
    };

    // Fetch the Game record
    let game_record = get(game_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found".into())))?;
    let game = game_record
        .entry()
        .to_app_option::<Game>()?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid Game entry".into())))?;

    // Ensure the game is InProgress or Finished
    match game.game_status {
        GameStatus::InProgress | GameStatus::Finished => (),
        _ => return Ok(ValidateCallbackResult::Invalid("Game is not active".into())),
    }

    // Ensure the player is part of the game
    if score.player != game.player_1 && score.player != game.player_2 {
        return Ok(ValidateCallbackResult::Invalid(
            "Score must be assigned to a player in the game".into(),
        ));
    }

    // Optionally, validate that the score points are within acceptable range
    if score.player_points > MAX_POINTS {
        return Ok(ValidateCallbackResult::Invalid(
            "Player points exceed the maximum allowed".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

// Helper function to get game hash by game_id
fn get_game_hash_by_id(game_id: &str) -> ExternResult<Option<ActionHash>> {
    // Fetch all games and find the one with matching game_id
    let game_links = get_links(
        GetLinksInputBuilder::try_new((), LinkTypes::GameUpdates)?.build(),
    )?;

    for link in game_links {
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
            if game.game_id == game_id {
                return Ok(Some(game_hash));
            }
        }
    }

    Ok(None)
}

pub fn validate_update_score(
    _action: Update,
    _score: Score,
    _original_action: EntryCreationAction,
    _original_score: Score,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_score(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original_score: Score,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_player_to_scores(
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
    let _score: crate::Score = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_player_to_scores(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_score_updates(
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
    let _score: crate::Score = record
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
    let _score: crate::Score = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_score_updates(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "ScoreUpdates links cannot be deleted".to_string(),
    ))
}
