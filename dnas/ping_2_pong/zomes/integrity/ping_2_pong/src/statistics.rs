use hdi::prelude::*;
use hdk::prelude::*;

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct Statistics {
    pub game_id: ActionHash,
    pub signal_latency: u32,
    pub score_validation_time: u32,
    pub dht_response_time: u32,
    pub network_delay: u32,
    pub timestamp: Timestamp,
}

pub fn validate_create_statistics(
    _action: EntryCreationAction,
    _statistics: Statistics,
) -> ExternResult<ValidateCallbackResult> {

    Ok(ValidateCallbackResult::Valid)
}


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
    _base_address: AnyLinkableHash,
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {

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
