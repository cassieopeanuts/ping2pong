use hdk::prelude::*;

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

pub mod game_validation;
pub mod player_validation;
pub mod score_validation; 
pub mod statistics_validation; 
pub mod presence_validation;

pub mod utils;

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
    AllPlayersAnchorToAgentPubKey, 
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
                                            EntryTypes::Score(score) => score_validation::validate_create_score(signed_action, score),
                                            EntryTypes::Statistics(statistics) => statistics_validation::validate_create_statistics(signed_action, statistics),
                                            EntryTypes::Presence(presence) => presence_validation::validate_create_presence(signed_action, presence),
                                            EntryTypes::AnchorPath(_) => Ok(ValidateCallbackResult::Valid), 
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
                    debug!("ValidationOp::Update for action {:?}: Validation delegated to specific entry type update validator.", update.original_action_address);
                    Ok(ValidateCallbackResult::Valid) 
                }
                // --- DELETE ENTRY ---
                Action::Delete(delete) => {
                    debug!("ValidationOp::Delete for deletes_address {:?}: Validation delegated to specific entry type delete validator.", delete.deletes_address);
                    Ok(ValidateCallbackResult::Valid) 
                }
                // --- CREATE LINK ---
                Action::CreateLink(create_link) => {
                     match LinkTypes::from_type(create_link.zome_index, create_link.link_type)? {
                        Some(link_type) => {
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
                                LinkTypes::AllPlayersAnchorToAgentPubKey => {
                                    if create_link.base_address.clone().into_entry_hash().is_none() {
                                        return Ok(ValidateCallbackResult::Invalid("AllPlayersAnchorToAgentPubKey base must be an EntryHash (anchor)".into()));
                                    }
                                    if create_link.target_address.clone().into_agent_pub_key().is_none() {
                                        return Ok(ValidateCallbackResult::Invalid("AllPlayersAnchorToAgentPubKey target must be an AgentPubKey".into()));
                                    }
                                    Ok(ValidateCallbackResult::Valid)
                                }
                            }
                        }
                        None => Ok(ValidateCallbackResult::Valid), 
                    }
                }
                 // --- DELETE LINK ---
                 Action::DeleteLink(delete_link) => {
                     debug!("ValidationOp::DeleteLink for link_add_address {:?}: Relying on default author validation.", delete_link.link_add_address);
                     Ok(ValidateCallbackResult::Valid)
                 }
                _ => Ok(ValidateCallbackResult::Valid),
            }
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

// --- Link Validations ---

fn validate_gameid_to_game_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    let _base_hash: AnyLinkableHash = create_link.base_address.clone(); 
    if create_link.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("GameIdToGame target must be an ActionHash".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player1_to_game_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    let base_agent = create_link.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("Player1ToGames base must be an AgentPubKey".into())))?;
    if create_link.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("Player1ToGames target must be an ActionHash".into()));
    }
    if create_link.author != base_agent {
         return Ok(ValidateCallbackResult::Invalid("Author of Player1ToGames link must be Player 1".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player2_to_game_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    let _base_agent = create_link.base_address.clone().into_agent_pub_key() 
         .ok_or(wasm_error!(WasmErrorInner::Guest("Player2ToGames base must be an AgentPubKey".into())))?;
    if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("Player2ToGames target must be an ActionHash".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_updates_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
     if create_link.base_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("GameUpdates base must be an ActionHash".into()));
     }
     if create_link.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("GameUpdates target must be an ActionHash".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_to_score_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
     if create_link.base_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToScores base must be a Game ActionHash".into()));
     }
     if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToScores target must be a Score ActionHash".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_to_statistics_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
      if create_link.base_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToStatistics base must be a Game ActionHash".into()));
     }
     if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToStatistics target must be a Statistics ActionHash".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_to_players_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
     let base_agent = create_link.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerToPlayers base must be an AgentPubKey".into())))?;
    if create_link.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("PlayerToPlayers target must be a Player ActionHash".into()));
    }
    if create_link.author != base_agent {
        return Ok(ValidateCallbackResult::Invalid("Author must be the Player themselves".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_playername_to_player_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
     if create_link.base_address.clone().into_entry_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("PlayerNameToPlayer base must be an EntryHash (Anchor)".into()));
     }
     if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerNameToPlayer target must be a Player ActionHash".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_updates_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
     if create_link.base_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerUpdates base must be an ActionHash".into()));
     }
     if create_link.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerUpdates target must be an ActionHash".into()));
     }
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

    Ok(ValidateCallbackResult::Valid)
}

fn validate_presence_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    if create_link.base_address.clone().into_entry_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("Base for Presence link must be 'presence' anchor hash".into()));
    }
    if create_link.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("Presence link target must be an ActionHash".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}