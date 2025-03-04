pub mod statistics;
pub use statistics::*;
pub mod score;
pub use score::*;
pub mod player;
pub use player::*;
pub mod game;
pub use game::*;

pub mod utils;
pub use utils::*;

use hdi::prelude::*;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Game(Game),
    Player(Player),
    Score(Score),
    Statistics(Statistics),
    Presence(Presence),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    Player1ToGames,
    Player2ToGames,
    GameUpdates,
    PlayerToPlayers,
    PlayerUpdates,
    PlayerToScores,
    ScoreUpdates,
    StatisticsUpdates,
    PlayerNameToPlayer,
    GameIdToGame,
    ScoreToPlayer,
    Presence,
}

// === Genesis and Agent Joining ===

#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_agent_joining(
    _agent_pub_key: AgentPubKey,
    _membrane_proof: &Option<MembraneProof>,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// === Unified Validation Callback ===

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        // For StoreEntry ops:
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::Game(game) => {
                    info!("Validating creation of game by agent: {:?}", action.author);
                    validate_create_game(EntryCreationAction::Create(action))
                }
                EntryTypes::Player(player) => {
                    validate_create_player(EntryCreationAction::Create(action), player)
                }
                EntryTypes::Score(score) => {
                    validate_create_score(EntryCreationAction::Create(action), score)
                }
                EntryTypes::Statistics(statistics) => {
                    validate_create_statistics(EntryCreationAction::Create(action), statistics)
                }
                // NEW: Handle Presence entries
                EntryTypes::Presence(presence) => {
                    // For presence, we simply require that the timestamp is non-zero.
                    if presence.timestamp > 0 {
                        Ok(ValidateCallbackResult::Valid)
                    } else {
                        Ok(ValidateCallbackResult::Invalid("Presence timestamp must be positive".into()))
                    }
                }
            },
            OpEntry::UpdateEntry { app_entry, action, .. } => match app_entry {
                EntryTypes::Game(game) => {
                    validate_create_game(EntryCreationAction::Update(action))
                }
                EntryTypes::Player(player) => {
                    validate_create_player(EntryCreationAction::Update(action), player)
                }
                EntryTypes::Score(score) => {
                    validate_create_score(EntryCreationAction::Update(action), score)
                }
                EntryTypes::Statistics(statistics) => {
                    validate_create_statistics(EntryCreationAction::Update(action), statistics)
                }
                // NEW: Disallow updates to Presence entries
                EntryTypes::Presence(_) => {
                    Ok(ValidateCallbackResult::Invalid("Presence entries are immutable".into()))
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // For RegisterUpdate ops:
        FlatOp::RegisterUpdate(update_entry) => match update_entry {
            OpUpdate::Entry { app_entry, action } => {
                let original_action = must_get_action(action.clone().original_action_address)?
                    .action()
                    .to_owned();
                let original_create_action = match EntryCreationAction::try_from(original_action) {
                    Ok(action) => action,
                    Err(e) => {
                        return Ok(ValidateCallbackResult::Invalid(format!(
                            "Expected EntryCreationAction from Action: {e:?}"
                        )));
                    }
                };
                match app_entry {
                    EntryTypes::Statistics(statistics) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_statistics = match Statistics::try_from(original_app_entry) {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(ValidateCallbackResult::Invalid(format!(
                                    "Expected Statistics from Record: {e:?}"
                                )));
                            }
                        };
                        validate_update_statistics(
                            action,
                            statistics,
                            original_create_action,
                            original_statistics,
                        )
                    }
                    EntryTypes::Score(score) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_score = match Score::try_from(original_app_entry) {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(ValidateCallbackResult::Invalid(format!(
                                    "Expected Score from Record: {e:?}"
                                )));
                            }
                        };
                        validate_update_score(action, score, original_create_action, original_score)
                    }
                    EntryTypes::Player(_player) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_player = match Player::try_from(original_app_entry) {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(ValidateCallbackResult::Invalid(format!(
                                    "Expected Player from Record: {e:?}"
                                )));
                            }
                        };
                        validate_update_player(action, original_create_action, original_player)
                    }
                    EntryTypes::Game(game) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_game = match Game::try_from(original_app_entry) {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(ValidateCallbackResult::Invalid(format!(
                                    "Expected Game from Record: {e:?}"
                                )));
                            }
                        };
                        validate_update_game(action, game, original_create_action, original_game)
                    }
                    // NEW: If an update is attempted on a Presence entry, disallow it.
                    EntryTypes::Presence(_) => {
                        Ok(ValidateCallbackResult::Invalid("Presence entries are immutable".into()))
                    }
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // For RegisterDelete ops:
        FlatOp::RegisterDelete(delete_entry) => {
            let original_action_hash = delete_entry.clone().action.deletes_address;
            let original_record = must_get_valid_record(original_action_hash)?;
            let original_record_action = original_record.action().clone();
            let original_action = match EntryCreationAction::try_from(original_record_action) {
                Ok(action) => action,
                Err(e) => {
                    return Ok(ValidateCallbackResult::Invalid(format!(
                        "Expected EntryCreationAction from Action: {e:?}"
                    )));
                }
            };
            let app_entry_type = match original_action.entry_type() {
                EntryType::App(app_entry_type) => app_entry_type,
                _ => return Ok(ValidateCallbackResult::Valid),
            };
            let entry = match original_record.entry().as_option() {
                Some(entry) => entry,
                None => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original record for a delete must contain an entry".to_string(),
                    ));
                }
            };
            let original_app_entry = match EntryTypes::deserialize_from_type(
                app_entry_type.zome_index,
                app_entry_type.entry_index,
                entry,
            )? {
                Some(app_entry) => app_entry,
                None => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original app entry must be one of the defined entry types for this zome".to_string(),
                    ));
                }
            };
            match original_app_entry {
                EntryTypes::Statistics(original_statistics) => {
                    validate_delete_statistics(delete_entry.clone().action, original_action, original_statistics)
                }
                EntryTypes::Score(original_score) => {
                    validate_delete_score(delete_entry.clone().action, original_action, original_score)
                }
                EntryTypes::Player(original_player) => {
                    validate_delete_player(delete_entry.clone().action, original_action, original_player)
                }
                EntryTypes::Game(original_game) => {
                    validate_delete_game(delete_entry.clone().action, original_action, original_game)
                }
                // Optionally, you can allow deletion of Presence entries.
                EntryTypes::Presence(_) => {
                    Ok(ValidateCallbackResult::Valid)
                }
            }
        },
        // For RegisterCreateLink ops:
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            tag,
            action,
        } => match link_type {
            LinkTypes::Player1ToGames => {
                validate_create_link_player_1_to_games(action, base_address, target_address, tag)
            }
            LinkTypes::Player2ToGames => {
                validate_create_link_player_2_to_games(action, base_address, target_address, tag)
            }
            LinkTypes::GameUpdates => {
                validate_create_link_game_updates(action, base_address, target_address, tag)
            }
            LinkTypes::PlayerToPlayers => {
                validate_create_link_player_to_players(action, base_address, tag)
            }
            LinkTypes::PlayerUpdates => {
                validate_create_link_player_updates(action, tag)
            }
            LinkTypes::PlayerToScores => {
                validate_create_link_player_to_scores(action, base_address, target_address, tag)
            }
            LinkTypes::ScoreUpdates => {
                validate_create_link_score_updates(action, base_address, target_address, tag)
            }
            LinkTypes::StatisticsUpdates => {
                validate_create_link_statistics_updates(action, base_address, target_address, tag)
            }
            LinkTypes::PlayerNameToPlayer => {
                validate_create_link_player_name_to_player(action, tag)
            }
            LinkTypes::GameIdToGame => {
                validate_create_link_game_id_to_game(action, base_address, target_address, tag)
            }
            LinkTypes::ScoreToPlayer => {
                validate_create_link_score_to_player(action, base_address, target_address, tag)
            }
            LinkTypes::Presence => {
                // Optionally validate creation of a Presence link.
                Ok(ValidateCallbackResult::Valid)
            }
        },
        // For RegisterDeleteLink ops:
        FlatOp::RegisterDeleteLink {
            link_type,
            base_address,
            target_address,
            tag,
            original_action,
            action,
        } => match link_type {
            LinkTypes::Player1ToGames => {
                validate_delete_link_player_1_to_games(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::Player2ToGames => {
                validate_delete_link_player_2_to_games(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::GameUpdates => {
                validate_delete_link_game_updates(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::PlayerToPlayers => {
                validate_delete_link_player_to_players(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::PlayerUpdates => {
                validate_delete_link_player_updates(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::PlayerToScores => {
                validate_delete_link_player_to_scores(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::ScoreUpdates => {
                validate_delete_link_score_updates(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::StatisticsUpdates => {
                validate_delete_link_statistics_updates(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::PlayerNameToPlayer => {
                validate_delete_link_player_name_to_player(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::GameIdToGame => {
                validate_delete_link_game_id_to_game(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::ScoreToPlayer => {
                validate_delete_link_score_to_player(action, original_action, base_address, target_address, tag)
            }
            LinkTypes::Presence => {
                // Optionally, allow deletion of Presence links.
                Ok(ValidateCallbackResult::Valid)
            }
        },
// For StoreRecord ops:
FlatOp::StoreRecord(store_record) => {
    match store_record {
        OpRecord::CreateEntry { app_entry, action } => match app_entry {
            EntryTypes::Game(game) => {
                info!("Validating creation of game by agent: {:?}", action.author);
                validate_create_game(EntryCreationAction::Create(action))
            }
            EntryTypes::Player(player) => {
                validate_create_player(EntryCreationAction::Create(action), player)
            }
            EntryTypes::Score(score) => {
                validate_create_score(EntryCreationAction::Create(action), score)
            }
            EntryTypes::Statistics(statistics) => {
                validate_create_statistics(EntryCreationAction::Create(action), statistics)
            }
            EntryTypes::Presence(presence) => {
                if presence.timestamp > 0 {
                    Ok(ValidateCallbackResult::Valid)
                } else {
                    Ok(ValidateCallbackResult::Invalid("Presence timestamp must be positive".into()))
                }
            }
        },
        OpRecord::UpdateEntry {
            original_action_hash,
            app_entry,
            action,
            ..
        } => {
            let original_record = must_get_valid_record(original_action_hash)?;
            let original_action = original_record.action().clone();
            let original_action = match original_action {
                Action::Create(create) => EntryCreationAction::Create(create),
                Action::Update(update) => EntryCreationAction::Update(update),
                _ => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original action for an update must be a Create or Update action".into(),
                    ));
                }
            };
            match app_entry {
                EntryTypes::Game(game) => {
                    let result = validate_create_game(
                        EntryCreationAction::Update(action.clone()),
                    )?;
                    if let ValidateCallbackResult::Valid = result {
                        let original_game: Option<Game> = original_record
                            .entry()
                            .to_app_option()
                            .map_err(|e| wasm_error!(e))?;
                        let original_game = match original_game {
                            Some(game) => game,
                            None => {
                                return Ok(ValidateCallbackResult::Invalid(
                                    "The updated entry type must be the same as the original entry type".into(),
                                ));
                            }
                        };
                        validate_update_game(action, game, original_action, original_game)
                    } else {
                        Ok(result)
                    }
                }
                EntryTypes::Player(player) => {
                    let result = validate_create_player(
                        EntryCreationAction::Update(action.clone()),
                        player.clone(),
                    )?;
                    if let ValidateCallbackResult::Valid = result {
                        let original_player: Option<Player> = original_record
                            .entry()
                            .to_app_option()
                            .map_err(|e| wasm_error!(e))?;
                        let original_player = match original_player {
                            Some(player) => player,
                            None => {
                                return Ok(ValidateCallbackResult::Invalid(
                                    "The updated entry type must be the same as the original entry type".into(),
                                ));
                            }
                        };
                        validate_update_player(action, original_action, original_player)
                    } else {
                        Ok(result)
                    }
                }
                EntryTypes::Score(score) => {
                    let result = validate_create_score(
                        EntryCreationAction::Update(action.clone()),
                        score.clone(),
                    )?;
                    if let ValidateCallbackResult::Valid = result {
                        let original_score: Option<Score> = original_record
                            .entry()
                            .to_app_option()
                            .map_err(|e| wasm_error!(e))?;
                        let original_score = match original_score {
                            Some(score) => score,
                            None => {
                                return Ok(ValidateCallbackResult::Invalid(
                                    "The updated entry type must be the same as the original entry type".into(),
                                ));
                            }
                        };
                        validate_update_score(action, score, original_action, original_score)
                    } else {
                        Ok(result)
                    }
                }
                EntryTypes::Statistics(statistics) => {
                    let result = validate_create_statistics(
                        EntryCreationAction::Update(action.clone()),
                        statistics.clone(),
                    )?;
                    if let ValidateCallbackResult::Valid = result {
                        let original_statistics: Option<Statistics> = original_record
                            .entry()
                            .to_app_option()
                            .map_err(|e| wasm_error!(e))?;
                        let original_statistics = match original_statistics {
                            Some(statistics) => statistics,
                            None => {
                                return Ok(ValidateCallbackResult::Invalid(
                                    "The updated entry type must be the same as the original entry type".into(),
                                ));
                            }
                        };
                        validate_update_statistics(action, statistics, original_action, original_statistics)
                    } else {
                        Ok(result)
                    }
                }
                // Disallow updates to Presence entries.
                EntryTypes::Presence(_) => {
                    Ok(ValidateCallbackResult::Invalid("Presence entries are immutable".into()))
                }
            }
        },
        OpRecord::DeleteEntry {
            original_action_hash,
            action,
            ..
        } => {
            let original_record = must_get_valid_record(original_action_hash)?;
            let original_action = original_record.action().clone();
            let original_action = match original_action {
                Action::Create(create) => EntryCreationAction::Create(create),
                Action::Update(update) => EntryCreationAction::Update(update),
                _ => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original action for a delete must be a Create or Update action".into(),
                    ));
                }
            };
            let app_entry_type = match original_action.entry_type() {
                EntryType::App(app_entry_type) => app_entry_type,
                _ => return Ok(ValidateCallbackResult::Valid),
            };
            let entry = match original_record.entry().as_option() {
                Some(entry) => entry,
                None => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original record for a delete must contain an entry".into(),
                    ));
                }
            };
            let original_app_entry = match EntryTypes::deserialize_from_type(
                app_entry_type.zome_index,
                app_entry_type.entry_index,
                entry,
            )? {
                Some(app_entry) => app_entry,
                None => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original app entry must be one of the defined entry types for this zome".into(),
                    ));
                }
            };
            match original_app_entry {
                EntryTypes::Game(original_game) => {
                    validate_delete_game(action, original_action, original_game)
                }
                EntryTypes::Player(original_player) => {
                    validate_delete_player(action, original_action, original_player)
                }
                EntryTypes::Score(original_score) => {
                    validate_delete_score(action, original_action, original_score)
                }
                EntryTypes::Statistics(original_statistics) => {
                    validate_delete_statistics(action, original_action, original_statistics)
                }
                EntryTypes::Presence(_) => {
                    // Allow deletion of Presence entries.
                    Ok(ValidateCallbackResult::Valid)
                }
            }
        },
        OpRecord::CreatePrivateEntry { .. } => Ok(ValidateCallbackResult::Valid),
        OpRecord::UpdatePrivateEntry { .. } => Ok(ValidateCallbackResult::Valid),
        OpRecord::CreateCapClaim { .. } => Ok(ValidateCallbackResult::Valid),
        OpRecord::CreateCapGrant { .. } => Ok(ValidateCallbackResult::Valid),
        OpRecord::UpdateCapClaim { .. } => Ok(ValidateCallbackResult::Valid),
        OpRecord::UpdateCapGrant { .. } => Ok(ValidateCallbackResult::Valid),
        OpRecord::Dna { .. } => Ok(ValidateCallbackResult::Valid),
        OpRecord::OpenChain { .. } => Ok(ValidateCallbackResult::Valid),
        OpRecord::CloseChain { .. } => Ok(ValidateCallbackResult::Valid),
        OpRecord::InitZomesComplete { .. } => Ok(ValidateCallbackResult::Valid),
        _ => Ok(ValidateCallbackResult::Valid),
    }
},
        FlatOp::RegisterAgentActivity(agent_activity) => match agent_activity {
            OpActivity::CreateAgent { agent, action } => {
                let previous_action = must_get_action(action.prev_action)?;
                match previous_action.action() {
                    Action::AgentValidationPkg(AgentValidationPkg { membrane_proof, .. }) =>
                        validate_agent_joining(agent, membrane_proof),
                    _ => Ok(ValidateCallbackResult::Invalid(
                        "The previous action for a CreateAgent must be an AgentValidationPkg".into(),
                    )),
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
    }
}
