use hdi::prelude::*;
use hdk::prelude::*;

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct Player {
    pub player_key: AgentPubKey,
    pub player_name: String,
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

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_player(
    action: Update,
    _original_action: EntryCreationAction,
    original_player: Player,
) -> ExternResult<ValidateCallbackResult> {
    // Players can only update their own profiles.
    if action.author != original_player.player_key {
        return Ok(ValidateCallbackResult::Invalid(
            "Players can only update their own profiles".into(),
        ));
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

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_player_to_players(
    _action: CreateLink,
    _base_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {

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
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {

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
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {

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
