use hdi::prelude::*;

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct Statistics {
    pub game_id: String,
    pub signal_latency: u32,
    pub score_validation_time: u32,
    pub dht_response_time: u32,
    pub network_delay: u32,
    pub timestamp: Timestamp,
}

pub fn validate_create_statistics(
    action: EntryCreationAction,
    statistics: Statistics,
) -> ExternResult<ValidateCallbackResult> {
    // Ensure the game_id exists
    let game_hash = match get_game_hash_by_id(&statistics.game_id)? {
        Some(hash) => hash,
        None => {
            return Ok(ValidateCallbackResult::Invalid(
                "Game ID does not exist".into(),
            ));
        }
    };

    let game_record = get(game_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found".into())))?;

    let game = game_record
        .entry()
        .to_app_option::<Game>()?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid game entry".into())))?;

    // Ensure game is Finished
    if game.game_status != GameStatus::Finished {
        return Ok(ValidateCallbackResult::Invalid(
            "Statistics can only be recorded for finished games".into(),
        ));
    }

    // Validate statistical metrics ranges
    if statistics.signal_latency > MAX_LATENCY {
        return Ok(ValidateCallbackResult::Invalid(
            "Signal latency exceeds maximum allowed value".into(),
        ));
    }

    if statistics.score_validation_time > MAX_SCORE_VALIDATION_TIME {
        return Ok(ValidateCallbackResult::Invalid(
            "Score validation time exceeds maximum allowed value".into(),
        ));
    }

    if statistics.dht_response_time > MAX_DHT_RESPONSE_TIME {
        return Ok(ValidateCallbackResult::Invalid(
            "DHT response time exceeds maximum allowed value".into(),
        ));
    }

    if statistics.network_delay > MAX_NETWORK_DELAY {
        return Ok(ValidateCallbackResult::Invalid(
            "Network delay exceeds maximum allowed value".into(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

// Define maximum allowed values as constants
const MAX_LATENCY: u32 = 10000; // in milliseconds
const MAX_SCORE_VALIDATION_TIME: u32 = 10000; // in milliseconds
const MAX_DHT_RESPONSE_TIME: u32 = 10000; // in milliseconds
const MAX_NETWORK_DELAY: u32 = 10000; // in milliseconds

pub fn validate_update_statistics(
    _action: Update,
    _statistics: Statistics,
    _original_action: EntryCreationAction,
    _original_statistics: Statistics,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_statistics(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original_statistics: Statistics,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_statistics_updates(
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
    let _statistics: crate::Statistics = record
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
    let _statistics: crate::Statistics = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_statistics_updates(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "StatisticsUpdates links cannot be deleted".to_string(),
    ))
}
