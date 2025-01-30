pub mod statistics;
pub use statistics::*;
pub mod score;
pub use score::*;
pub mod player;
pub use player::*;
pub mod game;
pub use game::*;
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
}

// Validation you perform during the genesis process. Nobody else on the network performs it, only you.
// There *is no* access to network calls in this callback
#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// Validation the network performs when you try to join, you can't perform this validation yourself as you are not a member yet.
// There *is* access to network calls in this function
pub fn validate_agent_joining(
    _agent_pub_key: AgentPubKey,
    _membrane_proof: &Option<MembraneProof>,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

// This is the unified validation callback for all entries and link types in this integrity zome
// Below is a match template for all of the variants of `DHT Ops` and entry and link types
// Holochain has already performed the following validation for you:
// - The action signature matches on the hash of its content and is signed by its author
// - The previous action exists, has a lower timestamp than the new action, and incremented sequence number
// - The previous action author is the same as the new action author
// - The timestamp of each action is after the DNA's origin time
// - AgentActivity authorities check that the agent hasn't forked their chain
// - The entry hash in the action matches the entry content
// - The entry type in the action matches the entry content
// - The entry size doesn't exceed the maximum entry size (currently 4MB)
// - Private entry types are not included in the Op content, and public entry types are
// - If the `Op` is an update or a delete, the original action exists and is a `Create` or `Update` action
// - If the `Op` is an update, the original entry exists and is of the same type as the new one
// - If the `Op` is a delete link, the original action exists and is a `CreateLink` action
// - Link tags don't exceed the maximum tag size (currently 1KB)
// - Countersigned entries include an action from each required signer
// You can read more about validation here: https://docs.rs/hdi/latest/hdi/index.html#data-validation
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::Game(game) => {
                    validate_create_game(EntryCreationAction::Create(action), game)
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
            },
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => match app_entry {
                EntryTypes::Game(game) => {
                    validate_create_game(EntryCreationAction::Update(action), game)
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
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(update_entry) => match update_entry {
            OpUpdate::Entry { app_entry, action } => {
                let original_action = must_get_action(action.clone().original_action_address)?
                    .action()
                    .to_owned();
                let original_create_action = match EntryCreationAction::try_from(original_action) {
                    Ok(action) => action,
                    Err(e) => {
                        return Ok(ValidateCallbackResult::Invalid(format!(
                            "Expected to get EntryCreationAction from Action: {e:?}"
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
                                    "Expected to get Statistics from Record: {e:?}"
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
                                    "Expected to get Score from Record: {e:?}"
                                )));
                            }
                        };
                        validate_update_score(action, score, original_create_action, original_score)
                    }
                    EntryTypes::Player(player) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_player = match Player::try_from(original_app_entry) {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(ValidateCallbackResult::Invalid(format!(
                                    "Expected to get Player from Record: {e:?}"
                                )));
                            }
                        };
                        validate_update_player(
                            action,
                            player,
                            original_create_action,
                            original_player,
                        )
                    }
                    EntryTypes::Game(game) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_game = match Game::try_from(original_app_entry) {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(ValidateCallbackResult::Invalid(format!(
                                    "Expected to get Game from Record: {e:?}"
                                )));
                            }
                        };
                        validate_update_game(action, game, original_create_action, original_game)
                    }
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDelete(delete_entry) => {
            let original_action_hash = delete_entry.clone().action.deletes_address;
            let original_record = must_get_valid_record(original_action_hash)?;
            let original_record_action = original_record.action().clone();
            let original_action = match EntryCreationAction::try_from(original_record_action) {
                Ok(action) => action,
                Err(e) => {
                    return Ok(ValidateCallbackResult::Invalid(format!(
                        "Expected to get EntryCreationAction from Action: {e:?}"
                    )));
                }
            };
            let app_entry_type = match original_action.entry_type() {
                EntryType::App(app_entry_type) => app_entry_type,
                _ => {
                    return Ok(ValidateCallbackResult::Valid);
                }
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
                        "Original app entry must be one of the defined entry types for this zome"
                            .to_string(),
                    ));
                }
            };
            match original_app_entry {
                EntryTypes::Statistics(original_statistics) => validate_delete_statistics(
                    delete_entry.clone().action,
                    original_action,
                    original_statistics,
                ),
                EntryTypes::Score(original_score) => validate_delete_score(
                    delete_entry.clone().action,
                    original_action,
                    original_score,
                ),
                EntryTypes::Player(original_player) => validate_delete_player(
                    delete_entry.clone().action,
                    original_action,
                    original_player,
                ),
                EntryTypes::Game(original_game) => validate_delete_game(
                    delete_entry.clone().action,
                    original_action,
                    original_game,
                ),
            }
        }
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
                validate_create_link_player_to_players(action, base_address, target_address, tag)
            }
            LinkTypes::PlayerUpdates => {
                validate_create_link_player_updates(action, base_address, target_address, tag)
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
        },
        FlatOp::RegisterDeleteLink {
            link_type,
            base_address,
            target_address,
            tag,
            original_action,
            action,
        } => match link_type {
            LinkTypes::Player1ToGames => validate_delete_link_player_1_to_games(
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
            LinkTypes::Player2ToGames => validate_delete_link_player_2_to_games(
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
            LinkTypes::GameUpdates => validate_delete_link_game_updates(
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
            LinkTypes::PlayerToPlayers => validate_delete_link_player_to_players(
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
            LinkTypes::PlayerUpdates => validate_delete_link_player_updates(
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
            LinkTypes::PlayerToScores => validate_delete_link_player_to_scores(
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
            LinkTypes::ScoreUpdates => validate_delete_link_score_updates(
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
            LinkTypes::StatisticsUpdates => validate_delete_link_statistics_updates(
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
        },
        FlatOp::StoreRecord(store_record) => {
            match store_record {
                // Complementary validation to the `StoreEntry` Op, in which the record itself is validated
                // If you want to optimize performance, you can remove the validation for an entry type here and keep it in `StoreEntry`
                // Notice that doing so will cause `must_get_valid_record` for this record to return a valid record even if the `StoreEntry` validation failed
                OpRecord::CreateEntry { app_entry, action } => match app_entry {
                    EntryTypes::Game(game) => {
                        validate_create_game(EntryCreationAction::Create(action), game)
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
                },
                // Complementary validation to the `RegisterUpdate` Op, in which the record itself is validated
                // If you want to optimize performance, you can remove the validation for an entry type here and keep it in `StoreEntry` and in `RegisterUpdate`
                // Notice that doing so will cause `must_get_valid_record` for this record to return a valid record even if the other validations failed
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
                                "Original action for an update must be a Create or Update action"
                                    .to_string(),
                            ));
                        }
                    };
                    match app_entry {
                        EntryTypes::Game(game) => {
                            let result = validate_create_game(
                                EntryCreationAction::Update(action.clone()),
                                game.clone(),
                            )?;
                            if let ValidateCallbackResult::Valid = result {
                                let original_game: Option<Game> = original_record
                                    .entry()
                                    .to_app_option()
                                    .map_err(|e| wasm_error!(e))?;
                                let original_game = match original_game {
                                    Some(game) => game,
                                    None => {
                                        return Ok(
                                            ValidateCallbackResult::Invalid(
                                                "The updated entry type must be the same as the original entry type"
                                                    .to_string(),
                                            ),
                                        );
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
                                        return Ok(
                                            ValidateCallbackResult::Invalid(
                                                "The updated entry type must be the same as the original entry type"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                };
                                validate_update_player(
                                    action,
                                    player,
                                    original_action,
                                    original_player,
                                )
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
                                        return Ok(
                                            ValidateCallbackResult::Invalid(
                                                "The updated entry type must be the same as the original entry type"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                };
                                validate_update_score(
                                    action,
                                    score,
                                    original_action,
                                    original_score,
                                )
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
                                        return Ok(
                                            ValidateCallbackResult::Invalid(
                                                "The updated entry type must be the same as the original entry type"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                };
                                validate_update_statistics(
                                    action,
                                    statistics,
                                    original_action,
                                    original_statistics,
                                )
                            } else {
                                Ok(result)
                            }
                        }
                    }
                }
                // Complementary validation to the `RegisterDelete` Op, in which the record itself is validated
                // If you want to optimize performance, you can remove the validation for an entry type here and keep it in `RegisterDelete`
                // Notice that doing so will cause `must_get_valid_record` for this record to return a valid record even if the `RegisterDelete` validation failed
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
                                "Original action for a delete must be a Create or Update action"
                                    .to_string(),
                            ));
                        }
                    };
                    let app_entry_type = match original_action.entry_type() {
                        EntryType::App(app_entry_type) => app_entry_type,
                        _ => {
                            return Ok(ValidateCallbackResult::Valid);
                        }
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
                            return Ok(
                                ValidateCallbackResult::Invalid(
                                    "Original app entry must be one of the defined entry types for this zome"
                                        .to_string(),
                                ),
                            );
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
                    }
                }
                // Complementary validation to the `RegisterCreateLink` Op, in which the record itself is validated
                // If you want to optimize performance, you can remove the validation for an entry type here and keep it in `RegisterCreateLink`
                // Notice that doing so will cause `must_get_valid_record` for this record to return a valid record even if the `RegisterCreateLink` validation failed
                OpRecord::CreateLink {
                    base_address,
                    target_address,
                    tag,
                    link_type,
                    action,
                } => match link_type {
                    LinkTypes::Player1ToGames => validate_create_link_player_1_to_games(
                        action,
                        base_address,
                        target_address,
                        tag,
                    ),
                    LinkTypes::Player2ToGames => validate_create_link_player_2_to_games(
                        action,
                        base_address,
                        target_address,
                        tag,
                    ),
                    LinkTypes::GameUpdates => {
                        validate_create_link_game_updates(action, base_address, target_address, tag)
                    }
                    LinkTypes::PlayerToPlayers => validate_create_link_player_to_players(
                        action,
                        base_address,
                        target_address,
                        tag,
                    ),
                    LinkTypes::PlayerUpdates => validate_create_link_player_updates(
                        action,
                        base_address,
                        target_address,
                        tag,
                    ),
                    LinkTypes::PlayerToScores => validate_create_link_player_to_scores(
                        action,
                        base_address,
                        target_address,
                        tag,
                    ),
                    LinkTypes::ScoreUpdates => validate_create_link_score_updates(
                        action,
                        base_address,
                        target_address,
                        tag,
                    ),
                    LinkTypes::StatisticsUpdates => validate_create_link_statistics_updates(
                        action,
                        base_address,
                        target_address,
                        tag,
                    ),
                },
                // Complementary validation to the `RegisterDeleteLink` Op, in which the record itself is validated
                // If you want to optimize performance, you can remove the validation for an entry type here and keep it in `RegisterDeleteLink`
                // Notice that doing so will cause `must_get_valid_record` for this record to return a valid record even if the `RegisterDeleteLink` validation failed
                OpRecord::DeleteLink {
                    original_action_hash,
                    base_address,
                    action,
                } => {
                    let record = must_get_valid_record(original_action_hash)?;
                    let create_link = match record.action() {
                        Action::CreateLink(create_link) => create_link.clone(),
                        _ => {
                            return Ok(ValidateCallbackResult::Invalid(
                                "The action that a DeleteLink deletes must be a CreateLink"
                                    .to_string(),
                            ));
                        }
                    };
                    let link_type = match LinkTypes::from_type(
                        create_link.zome_index,
                        create_link.link_type,
                    )? {
                        Some(lt) => lt,
                        None => {
                            return Ok(ValidateCallbackResult::Valid);
                        }
                    };
                    match link_type {
                        LinkTypes::Player1ToGames => validate_delete_link_player_1_to_games(
                            action,
                            create_link.clone(),
                            base_address,
                            create_link.target_address,
                            create_link.tag,
                        ),
                        LinkTypes::Player2ToGames => validate_delete_link_player_2_to_games(
                            action,
                            create_link.clone(),
                            base_address,
                            create_link.target_address,
                            create_link.tag,
                        ),
                        LinkTypes::GameUpdates => validate_delete_link_game_updates(
                            action,
                            create_link.clone(),
                            base_address,
                            create_link.target_address,
                            create_link.tag,
                        ),
                        LinkTypes::PlayerToPlayers => validate_delete_link_player_to_players(
                            action,
                            create_link.clone(),
                            base_address,
                            create_link.target_address,
                            create_link.tag,
                        ),
                        LinkTypes::PlayerUpdates => validate_delete_link_player_updates(
                            action,
                            create_link.clone(),
                            base_address,
                            create_link.target_address,
                            create_link.tag,
                        ),
                        LinkTypes::PlayerToScores => validate_delete_link_player_to_scores(
                            action,
                            create_link.clone(),
                            base_address,
                            create_link.target_address,
                            create_link.tag,
                        ),
                        LinkTypes::ScoreUpdates => validate_delete_link_score_updates(
                            action,
                            create_link.clone(),
                            base_address,
                            create_link.target_address,
                            create_link.tag,
                        ),
                        LinkTypes::StatisticsUpdates => validate_delete_link_statistics_updates(
                            action,
                            create_link.clone(),
                            base_address,
                            create_link.target_address,
                            create_link.tag,
                        ),
                    }
                }
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
        }
        FlatOp::RegisterAgentActivity(agent_activity) => match agent_activity {
            OpActivity::CreateAgent { agent, action } => {
                let previous_action = must_get_action(action.prev_action)?;
                match previous_action.action() {
                        Action::AgentValidationPkg(
                            AgentValidationPkg { membrane_proof, .. },
                        ) => validate_agent_joining(agent, membrane_proof),
                        _ => {
                            Ok(
                                ValidateCallbackResult::Invalid(
                                    "The previous action for a `CreateAgent` action must be an `AgentValidationPkg`"
                                        .to_string(),
                                ),
                            )
                        }
                    }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
    }
}
