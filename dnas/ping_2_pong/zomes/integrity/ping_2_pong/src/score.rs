use hdi::prelude::*;
use hdk::prelude::*;

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct Score {
    pub game_id: ActionHash,
    pub player: AgentPubKey,
    pub player_points: u32,
}

pub fn validate_create_score(
    _action: EntryCreationAction,
    _score: Score,
) -> ExternResult<ValidateCallbackResult> {

    Ok(ValidateCallbackResult::Valid)
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
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {

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
    _base_address: AnyLinkableHash,
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {

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

// Stub for validating creation of a ScoreToPlayer link.
pub fn validate_create_link_score_to_player(
    _action: CreateLink,
    _base_address: AnyLinkableHash,
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: Implement detailed validation.
    Ok(ValidateCallbackResult::Valid)
}

// Stub for validating deletion of a ScoreToPlayer link.
pub fn validate_delete_link_score_to_player(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: Implement detailed validation.
    Ok(ValidateCallbackResult::Valid)
}
