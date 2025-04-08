// ping_2_pong/dnas/ping_2_pong/zomes/integrity/ping_2_pong/src/lib.rs
use hdk::prelude::*;

// Import entry definitions
pub mod game;
pub use game::Game;
pub mod player;
pub use player::Player;
pub mod score;
pub use score::Score;
pub mod statistics;
pub use statistics::Statistics;
pub mod presence;
pub use presence::Presence;
pub mod anchor_path;
pub use anchor_path::AnchorPath;

// Import validation functions for entries
pub mod game_validation;
pub mod player_validation;
pub mod score_validation;
pub mod statistics_validation;
pub mod presence_validation;

// Import utils like anchor_for (used only by link validation helpers below)
pub mod utils;

// Define EntryTypes enum with Serde derives
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum EntryTypes {
    #[entry_type(visibility = "public")]
    Game(Game),
    #[entry_type(visibility = "public")]
    Player(Player),
    #[entry_type(visibility = "public")]
    Score(Score),
    #[entry_type(visibility = "public")]
    Statistics(Statistics),
    #[entry_type(visibility = "public")]
    Presence(Presence),
    #[entry_type(visibility = "public")]
    AnchorPath(AnchorPath),
}

// Define LinkTypes enum with Serde derives
#[hdk_link_types]
#[derive(Serialize, Deserialize, Hash)]
pub enum LinkTypes {
    GameIdToGame,
    Player1ToGames,
    Player2ToGames,
    GameUpdates,
    GameToScores,
    GameToStatistics,
    PlayerToPlayers,
    PlayerNameToPlayer,
    PlayerUpdates,
    PlayerToScores,
    Presence,
}

// Main Validation Callback
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op {
        Op::StoreRecord(store_record) => {
            let record = store_record.record;
            let signed_action = record.signed_action();

            match record.action().clone() {
                // --- CREATE ENTRY ---
                Action::Create(create) => {
                    if let EntryType::App(app_entry_type) = create.entry_type {
                         match record.entry().as_option() {
                            Some(entry) => {
                                match EntryTypes::deserialize_from_type(app_entry_type.zome_index, app_entry_type.entry_index, entry)? {
                                    Some(entry_types) => {
                                        match entry_types {
                                            EntryTypes::Game(game) => game_validation::validate_create_game(signed_action, game),
                                            EntryTypes::Player(player) => player_validation::validate_create_player(signed_action, player),
                                            EntryTypes::Score(score) => score_validation::validate_create_score(signed_action, score), // Remember this was simplified
                                            EntryTypes::Statistics(statistics) => statistics_validation::validate_create_statistics(signed_action, statistics), // Remember this was simplified
                                            EntryTypes::Presence(presence) => presence_validation::validate_create_presence(signed_action, presence),
                                            EntryTypes::AnchorPath(_) => Ok(ValidateCallbackResult::Valid), // Anchor paths are structural
                                        }
                                    }
                                    None => Ok(ValidateCallbackResult::Valid),
                                }
                            }
                            None => Ok(ValidateCallbackResult::Invalid("Create action Record is missing Entry".to_string())),
                        }
                    } else { Ok(ValidateCallbackResult::Valid) }
                }
                // --- UPDATE ENTRY ---
                 Action::Update(update) => {
                    match record.entry().as_option() {
                        Some(updated_entry_data) => {
                            // We need the original entry to call the specific update validator
                            match get(update.original_action_address.clone(), GetOptions::default())? {
                                Some(original_record) => {
                                     match original_record.entry().as_option() {
                                         Some(original_entry_data) => {
                                            if let Some(EntryType::App(app_entry_type)) = original_record.action().entry_type() {
                                                 // Deserialize both entries
                                                match (EntryTypes::deserialize_from_type(app_entry_type.zome_index, app_entry_type.entry_index, updated_entry_data)?,
                                                       EntryTypes::deserialize_from_type(app_entry_type.zome_index, app_entry_type.entry_index, original_entry_data)?) {
                                                    (Some(updated_entry_type), Some(original_entry_type)) => {
                                                        // Check types match before calling specific validation
                                                        match (updated_entry_type, original_entry_type) {
                                                            (EntryTypes::Game(game), EntryTypes::Game(original_game)) => game_validation::validate_update_game(signed_action, game, &original_game),
                                                            (EntryTypes::Player(player), EntryTypes::Player(original_player)) => player_validation::validate_update_player(signed_action, player, &original_player),
                                                            (EntryTypes::Score(_), _) | (EntryTypes::Statistics(_), _) | (EntryTypes::Presence(_), _) | (EntryTypes::AnchorPath(_), _) => Ok(ValidateCallbackResult::Invalid("Updates not allowed for this entry type".into())),
                                                            _ => Ok(ValidateCallbackResult::Invalid("Mismatched entry types during update validation".into())) // Should not happen if indices match
                                                        }
                                                    }
                                                    _ => Ok(ValidateCallbackResult::Valid) // Could not deserialize one or both entries as expected types
                                                }
                                            } else { Ok(ValidateCallbackResult::Valid) } // Original not an App entry
                                         }
                                         None => Ok(ValidateCallbackResult::Invalid("Original record for update is missing Entry".to_string()))
                                     }
                                }
                                None => Ok(ValidateCallbackResult::Invalid("Original record not found for update validation".to_string()))
                            }
                        }
                        None => Ok(ValidateCallbackResult::Invalid("Update action Record is missing Entry".to_string()))
                    }
                }
                // --- DELETE ENTRY ---
                Action::Delete(delete) => {
                    // We need the original entry to call the specific delete validator
                    match get(delete.deletes_address.clone(), GetOptions::default())? {
                         Some(original_record) => {
                              match original_record.entry().as_option() {
                                 Some(original_entry_data) => {
                                      if let Some(EntryType::App(app_entry_type)) = original_record.action().entry_type() {
                                         match EntryTypes::deserialize_from_type(app_entry_type.zome_index, app_entry_type.entry_index, original_entry_data)? {
                                            Some(original_entry_type) => {
                                                 match original_entry_type {
                                                     EntryTypes::Game(game) => game_validation::validate_delete_game(signed_action, game),
                                                     EntryTypes::Player(player) => player_validation::validate_delete_player(signed_action, player),
                                                     EntryTypes::Score(_) | EntryTypes::Statistics(_) | EntryTypes::Presence(_) | EntryTypes::AnchorPath(_) => {
                                                         Ok(ValidateCallbackResult::Invalid("Deletes not allowed for this entry type".into()))
                                                     }
                                                 }
                                            }
                                            None => Ok(ValidateCallbackResult::Valid) // Type not known to this zome
                                        }
                                    } else { Ok(ValidateCallbackResult::Valid) } // Not an App entry
                                 }
                                 None => Ok(ValidateCallbackResult::Invalid("Original record for delete is missing Entry".to_string()))
                              }
                         }
                         None => Ok(ValidateCallbackResult::Invalid("Original record not found for delete validation".to_string()))
                    }
                }
                // --- CREATE LINK ---
                Action::CreateLink(create_link) => {
                     match LinkTypes::from_type(create_link.zome_index, create_link.link_type)? {
                        Some(link_type) => {
                             // Call the simplified link validation functions below
                             match link_type {
                                LinkTypes::GameIdToGame => validate_gameid_to_game_link(&create_link),
                                LinkTypes::Player1ToGames => validate_player1_to_game_link(&create_link),
                                LinkTypes::Player2ToGames => validate_player2_to_game_link(&create_link),
                                LinkTypes::GameUpdates => validate_game_updates_link(&create_link),
                                LinkTypes::GameToScores => validate_game_to_score_link(&create_link),
                                LinkTypes::GameToStatistics => validate_game_to_statistics_link(&create_link),
                                LinkTypes::PlayerToPlayers => validate_player_to_players_link(&create_link),
                                LinkTypes::PlayerNameToPlayer => validate_playername_to_player_link(&create_link),
                                LinkTypes::PlayerUpdates => validate_player_updates_link(&create_link),
                                LinkTypes::PlayerToScores => validate_player_to_scores_link(&create_link),
                                LinkTypes::Presence => validate_presence_link(&create_link),
                            }
                        }
                        None => Ok(ValidateCallbackResult::Valid), // Allow unknown link types
                    }
                }
                 // --- DELETE LINK ---
                 Action::DeleteLink(delete_link) => {
                     // Fetch the original CreateLink action to check author
                     match get(delete_link.link_add_address.clone(), GetOptions::default())? {
                         Some(create_link_record) => {
                              if let Action::CreateLink(create_link) = create_link_record.action().clone() {
                                  if create_link.author != delete_link.author {
                                       return Ok(ValidateCallbackResult::Invalid("DeleteLink author must match CreateLink author".to_string()));
                                  }
                                  // Add specific delete validation logic here if needed, e.g.
                                  // match LinkTypes::from_type(create_link.zome_index, create_link.link_type)? {
                                  //     Some(LinkTypes::GameUpdates) => Ok(ValidateCallbackResult::Invalid("GameUpdates links cannot be deleted".into())),
                                  //     _ => Ok(ValidateCallbackResult::Valid),
                                  // }
                                  Ok(ValidateCallbackResult::Valid) // Default allow delete by author
                              } else { Ok(ValidateCallbackResult::Invalid("Original action for DeleteLink is not CreateLink".to_string())) }
                         }
                         None => Ok(ValidateCallbackResult::Invalid("Original CreateLink action not found for DeleteLink validation".to_string()))
                     }
                 }
                // --- Other Actions ---
                _ => Ok(ValidateCallbackResult::Valid),
            }
        }
        // Handle other Ops if necessary, otherwise allow
        _ => Ok(ValidateCallbackResult::Valid),
    }
}


// --- Simplified Link Validations (No `get` calls inside) ---

fn validate_gameid_to_game_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be AnyLinkableHash (can be EntryHash)
    let _base_hash: AnyLinkableHash = create_link.base_address.clone(); // Type already correct
    // Target Check: Must be an ActionHash
    if create_link.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("GameIdToGame target must be an ActionHash".into()));
    }
    // Author Check: Allow anyone
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player1_to_game_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be an AgentPubKey
    let base_agent = create_link.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("Player1ToGames base must be an AgentPubKey".into())))?;
    // Target Check: Must be ActionHash
    if create_link.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("Player1ToGames target must be an ActionHash".into()));
    }
    // Author Check: Must be the Agent from the base address
    if create_link.author != base_agent {
         return Ok(ValidateCallbackResult::Invalid("Author of Player1ToGames link must be Player 1".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player2_to_game_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be an AgentPubKey
    let base_agent = create_link.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("Player2ToGames base must be an AgentPubKey".into())))?;
    // Target Check: Must be ActionHash
    if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("Player2ToGames target must be an ActionHash".into()));
    }
    // Author Check: Must be the Agent from the base address
     if create_link.author != base_agent {
         return Ok(ValidateCallbackResult::Invalid("Author of Player2ToGames link must be Player 2".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_updates_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be ActionHash
     if create_link.base_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("GameUpdates base must be an ActionHash".into()));
     }
    // Target Check: Must be ActionHash
     if create_link.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("GameUpdates target must be an ActionHash".into()));
     }
    // Note: Cannot validate author or target relationship without get calls
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_to_score_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be ActionHash
     if create_link.base_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToScores base must be a Game ActionHash".into()));
     }
    // Target Check: Must be ActionHash
     if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToScores target must be a Score ActionHash".into()));
     }
    // Note: Cannot validate score belongs to game without get calls
    // Author Check: Allow anyone? Or check against game players (needs get)? Allow for now.
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_to_statistics_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be ActionHash
      if create_link.base_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToStatistics base must be a Game ActionHash".into()));
     }
    // Target Check: Must be ActionHash
     if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToStatistics target must be a Statistics ActionHash".into()));
     }
    // Note: Cannot validate stats belong to game or author without get calls
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_to_players_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be an AgentPubKey
     let base_agent = create_link.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerToPlayers base must be an AgentPubKey".into())))?;
    // Target Check: Must be ActionHash
    if create_link.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("PlayerToPlayers target must be a Player ActionHash".into()));
    }
    // Author Check: Must be the Agent from the base address
    if create_link.author != base_agent {
        return Ok(ValidateCallbackResult::Invalid("Author must be the Player themselves".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_playername_to_player_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be AnyLinkableHash (EntryHash)
     if create_link.base_address.clone().into_entry_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("PlayerNameToPlayer base must be an EntryHash (Anchor)".into()));
     }
    // Target Check: Must be ActionHash
     if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerNameToPlayer target must be a Player ActionHash".into()));
     }
     // Note: Cannot validate anchor matches player name or author without get calls
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_updates_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be ActionHash
     if create_link.base_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerUpdates base must be an ActionHash".into()));
     }
    // Target Check: Must be ActionHash
     if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerUpdates target must be an ActionHash".into()));
     }
     // Note: Cannot validate author or target relationship without get calls
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_to_scores_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be an AgentPubKey
     let base_agent = create_link.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerToScores base must be an AgentPubKey".into())))?;
    // Target Check: Must be ActionHash
    if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerToScores target must be a Score ActionHash".into()));
     }
    // Author Check: Must be the Agent from the base address
    if create_link.author != base_agent {
        return Ok(ValidateCallbackResult::Invalid("Author must be the Player whose score it is".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_presence_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // Base Check: Must be AnyLinkableHash (EntryHash)
    if create_link.base_address.clone().into_entry_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("Base for Presence link must be 'presence' anchor hash".into()));
    }
    // Target Check: Must be ActionHash
    if create_link.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("Presence link target must be an ActionHash".into()));
    }
    // Note: Cannot validate author without get call to retrieve Presence entry's agent_pubkey
    Ok(ValidateCallbackResult::Valid)
}