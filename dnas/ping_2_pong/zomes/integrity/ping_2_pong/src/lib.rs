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

// Import validation functions
pub mod game_validation;
pub mod player_validation;
pub mod score_validation;
pub mod statistics_validation;
pub mod presence_validation;
// pub mod anchor_path_validation;

pub mod utils;

// Define EntryTypes enum
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
#[derive(Serialize, Deserialize, Clone, PartialEq)] // Added derives
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

// Define LinkTypes enum
#[hdk_link_types]
#[derive(Serialize, Deserialize, Hash)] // Added derives
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
        // === ENTRY OPERATIONS (via StoreRecord) ===
        Op::StoreRecord(store_record) => {
            let record = store_record.record;
            let signed_action = record.signed_action();

            match record.action().clone() {
                // --- CREATE ---
                Action::Create(create) => {
                    // Directly match on create.entry_type (FIXED: It's not an Option)
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
                                    None => Ok(ValidateCallbackResult::Valid), // Allow other app entry types
                                }
                            }
                            None => Ok(ValidateCallbackResult::Invalid("Create action Record is missing Entry".to_string())),
                        }
                    } else {
                        Ok(ValidateCallbackResult::Valid) // Not an App entry type
                    }
                }

                // --- UPDATE ---
                // ... (Update logic remains the same - it relies on fetching original action) ...
                 Action::Update(update) => {
                    match record.entry().as_option() {
                        Some(updated_entry_data) => {
                            // Still need original type info for deserialization
                            if let Some(original_entry_type) = get_action_entry_type(&update.original_action_address)? {
                                if let EntryType::App(app_entry_type) = original_entry_type {
                                    match EntryTypes::deserialize_from_type(app_entry_type.zome_index, app_entry_type.entry_index, updated_entry_data)? {
                                        Some(updated_entry_type) => {
                                             match updated_entry_type {
                                                 EntryTypes::Game(game) => game_validation::validate_update_game(signed_action, game),
                                                 EntryTypes::Player(player) => player_validation::validate_update_player(signed_action, player),
                                                 EntryTypes::Score(_) | EntryTypes::Statistics(_) | EntryTypes::Presence(_) | EntryTypes::AnchorPath(_) => {
                                                     Ok(ValidateCallbackResult::Invalid("Updates not allowed for this entry type".into()))
                                                 }
                                             }
                                        }
                                        None => Ok(ValidateCallbackResult::Valid),
                                    }
                                } else { Ok(ValidateCallbackResult::Valid) }
                            } else { Ok(ValidateCallbackResult::Invalid("Could not determine original entry type for update".into()))}
                        }
                        None => Ok(ValidateCallbackResult::Invalid("Update action Record is missing Entry".to_string()))
                    }
                }

                // --- DELETE ---
                // ... (Delete logic remains the same - it relies on fetching original action/entry) ...
                Action::Delete(delete) => {
                    match get_entry_for_action(&delete.deletes_address)? {
                        Some((original_entry_data, app_entry_type)) => {
                             match EntryTypes::deserialize_from_type(app_entry_type.zome_index, app_entry_type.entry_index, &original_entry_data)? {
                                Some(original_entry_type) => {
                                     match original_entry_type {
                                         EntryTypes::Game(game) => game_validation::validate_delete_game(signed_action, game),
                                         EntryTypes::Player(player) => player_validation::validate_delete_player(signed_action, player),
                                         EntryTypes::Score(_) | EntryTypes::Statistics(_) | EntryTypes::Presence(_) | EntryTypes::AnchorPath(_) => {
                                             Ok(ValidateCallbackResult::Invalid("Deletes not allowed for this entry type".into()))
                                         }
                                     }
                                }
                                None => Ok(ValidateCallbackResult::Valid),
                            }
                        }
                         None => Ok(ValidateCallbackResult::Invalid("Could not retrieve original entry for delete validation".to_string()))
                    }
                }


                // --- CREATE LINK ---
                // ... (Create Link logic remains the same) ...
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
                            }
                        }
                        None => Ok(ValidateCallbackResult::Valid),
                    }
                }


                 // --- DELETE LINK ---
                 // ... (Delete Link logic remains the same) ...
                 Action::DeleteLink(delete_link) => {
                     match get(delete_link.link_add_address.clone(), GetOptions::default())? {
                         Some(create_link_record) => {
                              if let Action::CreateLink(create_link) = create_link_record.action().clone() {
                                  if create_link.author != delete_link.author {
                                       return Ok(ValidateCallbackResult::Invalid("DeleteLink author must match CreateLink author".to_string()));
                                  }
                                  Ok(ValidateCallbackResult::Valid)
                              } else {
                                   Ok(ValidateCallbackResult::Invalid("Original action for DeleteLink is not CreateLink".to_string()))
                              }
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

// ... (Helper functions and Link Validations remain the same) ...

// Helper to get the full Record using ActionHash
fn must_get_record(action_hash: ActionHash) -> ExternResult<Record> {
    get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!("Record not found for action hash: {}", action_hash))))
}

// Helper to get original Action details to find entry type
#[allow(dead_code)] // May not be needed if validation logic changes
fn get_action_entry_type(action_hash: &ActionHash) -> ExternResult<Option<EntryType>> {
     let record = must_get_record(action_hash.clone())?;
     Ok(record.action().entry_type().cloned())
}

// Helper to get original entry data and type definition
#[allow(dead_code)] // May not be needed if validation logic changes
fn get_entry_for_action(action_hash: &ActionHash) -> ExternResult<Option<(Entry, AppEntryDef)>> {
    let record = must_get_record(action_hash.clone())?;
    match (record.action().entry_type(), record.entry().as_option()) {
        (Some(EntryType::App(app_entry_type)), Some(entry)) => Ok(Some((entry.clone(), app_entry_type.clone()))),
        _ => Ok(None),
    }
}

// Helper to extract Entry from Option<Record> and convert to specific type T
fn extract_entry_as<T>(record_option: Option<Record>, type_name: &str) -> ExternResult<T>
where
    T: TryFrom<Entry, Error = WasmError> + Clone,
{
    let record = record_option.ok_or_else(|| wasm_error!(WasmErrorInner::Guest(format!("Target record for {} link not found", type_name))))?;
    let entry = record.entry().as_option()
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(format!("Target {} record missing entry data", type_name))))?
        .clone();
    T::try_from(entry)
}

// --- Specific Link Validations (Implementations) ---

fn validate_gameid_to_game_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be the known "games" anchor hash
    let expected_base_hash = utils::anchor_for("games")?; // Type AnyLinkableHash (EntryHash)
    if create_link.base_address != expected_base_hash {
        return Ok(ValidateCallbackResult::Invalid("Base for GameIdToGame must be 'games' anchor hash".into()));
    }

    // 2. Target Check: Must be an ActionHash pointing to a valid Game entry
    let target_ah = create_link.target_address.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("GameIdToGame target must be an ActionHash".into())))?;
    let target_record = get(target_ah.clone(), GetOptions::default())?; // Fetch target record
    let _game: Game = extract_entry_as(target_record, "Game")?; // Check type

    // 3. Author Check: Allow anyone (system link)
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player1_to_game_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be an AgentPubKey
    let base_agent = create_link.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("Player1ToGames base must be an AgentPubKey".into())))?;

    // 2. Target Check: Must be ActionHash pointing to valid Game
    let target_ah = create_link.target_address.clone().into_action_hash()
         .ok_or(wasm_error!(WasmErrorInner::Guest("Player1ToGames target must be an ActionHash".into())))?;
    let target_record = get(target_ah.clone(), GetOptions::default())?;
    let game: Game = extract_entry_as(target_record, "Game")?;

    // 3. Relationship Check: Base agent must be player 1 in the game
    if game.player_1 != base_agent {
        return Ok(ValidateCallbackResult::Invalid("Link base AgentPubKey does not match Game's player_1".into()));
    }

    // 4. Author Check: Must be Player 1
    if create_link.author != base_agent {
         return Ok(ValidateCallbackResult::Invalid("Author of Player1ToGames link must be Player 1".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player2_to_game_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be an AgentPubKey
    let base_agent = create_link.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("Player2ToGames base must be an AgentPubKey".into())))?;

    // 2. Target Check: Must be ActionHash pointing to valid Game
    let target_ah = create_link.target_address.clone().into_action_hash()
         .ok_or(wasm_error!(WasmErrorInner::Guest("Player2ToGames target must be an ActionHash".into())))?;
    let target_record = get(target_ah.clone(), GetOptions::default())?;
    let game: Game = extract_entry_as(target_record, "Game")?;

    // 3. Relationship Check: Base agent must be player 2 in the game (and player 2 must exist)
    match &game.player_2 {
        Some(p2) if p2 == &base_agent => { /* Checks pass */ },
        _ => return Ok(ValidateCallbackResult::Invalid("Link base AgentPubKey does not match Game's player_2 or player_2 is not set".into()))
    }

    // 4. Author Check: Must be Player 2
    if create_link.author != base_agent {
         return Ok(ValidateCallbackResult::Invalid("Author of Player2ToGames link must be Player 2".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_updates_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be ActionHash of original Game create
     let base_ah = create_link.base_address.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("GameUpdates base must be an ActionHash".into())))?;
    // Check if base record exists and contains a Game
    let base_record = get(base_ah.clone(), GetOptions::default())?;
    let original_game: Game = extract_entry_as(base_record, "Original Game")?;

    // 2. Target Check: Must be ActionHash of update action
     let target_ah = create_link.target_address.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("GameUpdates target must be an ActionHash".into())))?;
     let target_record = must_get_record(target_ah)?; // Use must_get_record for ActionHash

    // 3. Relationship Check: Target action must be an Update of Base action
     match target_record.action().clone() {
         Action::Update(update) => {
             if update.original_action_address != base_ah {
                return Ok(ValidateCallbackResult::Invalid("GameUpdates link target is not an update of the base".into()));
             }
             // Check target entry is also a Game
             let _updated_game: Game = extract_entry_as(Some(target_record), "Updated Game")?;
         }
         _ => return Ok(ValidateCallbackResult::Invalid("GameUpdates link target is not an Update action".into()))
     }

     // 4. Author Check: Must be one of the players from the original game
     if create_link.author != original_game.player_1 && original_game.player_2.as_ref() != Some(&create_link.author) {
         return Ok(ValidateCallbackResult::Invalid("Author of GameUpdates link must be a player in the game".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_to_score_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be ActionHash of original Game create
     let base_game_ah = create_link.base_address.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("GameToScores base must be a Game ActionHash".into())))?;
     let base_game_record = get(base_game_ah.clone(), GetOptions::default())?;
     let game: Game = extract_entry_as(base_game_record, "Game")?;

    // 2. Target Check: Must be ActionHash of Score create
     let target_score_ah = create_link.target_address.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("GameToScores target must be a Score ActionHash".into())))?;
    let target_score_record = get(target_score_ah.clone(), GetOptions::default())?;
    let score: Score = extract_entry_as(target_score_record, "Score")?;

    // 3. Relationship Check: Score game_id must match base Game ActionHash & player must be in game
     if score.game_id != base_game_ah {
         return Ok(ValidateCallbackResult::Invalid("Score's game_id does not match linked Game action hash".into()));
     }
     if score.player != game.player_1 && game.player_2.as_ref() != Some(&score.player) {
         return Ok(ValidateCallbackResult::Invalid("Score's player was not in the linked Game".into()));
     }

    // 4. Author Check: Allow anyone (consistent with coordinator)
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_to_statistics_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be ActionHash of original Game create
      let base_game_ah = create_link.base_address.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("GameToStatistics base must be a Game ActionHash".into())))?;
     let base_game_record = get(base_game_ah.clone(), GetOptions::default())?;
     let _game: Game = extract_entry_as(base_game_record, "Game")?; // Check base is Game

    // 2. Target Check: Must be ActionHash of Statistics create
     let target_stats_ah = create_link.target_address.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("GameToStatistics target must be a Statistics ActionHash".into())))?;
     let target_stats_record = get(target_stats_ah.clone(), GetOptions::default())?;
     let stats: Statistics = extract_entry_as(target_stats_record, "Statistics")?;

    // 3. Relationship Check: Statistics game_id must match base Game ActionHash
     if stats.game_id != base_game_ah {
         return Ok(ValidateCallbackResult::Invalid("Statistics' game_id does not match linked Game action hash".into()));
     }

    // 4. Author Check: Must be one of the players in the game
    if create_link.author != _game.player_1 && _game.player_2.as_ref() != Some(&create_link.author) {
         return Ok(ValidateCallbackResult::Invalid("Author of GameToStatistics link must be a player in the game".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_to_players_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be an AgentPubKey
     let base_agent = create_link.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerToPlayers base must be an AgentPubKey".into())))?;

    // 2. Target Check: Must be ActionHash of Player create
    let target_player_ah = create_link.target_address.clone().into_action_hash()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerToPlayers target must be a Player ActionHash".into())))?;
    let target_player_record = get(target_player_ah.clone(), GetOptions::default())?;
    let player: Player = extract_entry_as(target_player_record, "Player")?;

    // 3. Relationship Check: Player's key must match link base
     if player.player_key != base_agent {
         return Ok(ValidateCallbackResult::Invalid("Player's key does not match link base AgentPubKey".into()));
     }

    // 4. Author Check: Must be the player themselves
    if create_link.author != base_agent {
        return Ok(ValidateCallbackResult::Invalid("Author must be the Player themselves".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_playername_to_player_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be Name Anchor hash (EntryHash)
     let base_anchor_hash = create_link.base_address.clone().into_entry_hash()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerNameToPlayer base must be an EntryHash (Anchor)".into())))?;

    // 2. Target Check: Must be ActionHash of Player create
     let target_player_ah = create_link.target_address.clone().into_action_hash()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerNameToPlayer target must be a Player ActionHash".into())))?;
    let target_player_record = get(target_player_ah.clone(), GetOptions::default())?;
    let player: Player = extract_entry_as(target_player_record, "Player")?;

    // 3. Relationship Check: Base anchor hash should match hash derived from player name
     let expected_base_hash = utils::anchor_for(&player.player_name.to_lowercase())?
         .into_entry_hash() // Convert AnyLinkableHash to EntryHash for comparison
         .ok_or(wasm_error!(WasmErrorInner::Guest("Could not get EntryHash from name anchor".into())))?;
     if base_anchor_hash != expected_base_hash {
         return Ok(ValidateCallbackResult::Invalid("Base anchor hash does not match hash derived from player name".into()));
     }

    // 4. Author Check: Must be the player themselves
    if create_link.author != player.player_key {
         return Ok(ValidateCallbackResult::Invalid("Author must be the Player".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_updates_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be ActionHash of original Player create
     let base_ah = create_link.base_address.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerUpdates base must be an ActionHash".into())))?;
    let base_record = get(base_ah.clone(), GetOptions::default())?;
    let original_player: Player = extract_entry_as(base_record, "Original Player")?;

    // 2. Target Check: Must be ActionHash of update action
     let target_ah = create_link.target_address.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerUpdates target must be an ActionHash".into())))?;
     let target_record = must_get_record(target_ah)?; // Use must_get_record for ActionHash

    // 3. Relationship Check: Target action must be an Update of Base action
     match target_record.action().clone() {
         Action::Update(update) => {
             if update.original_action_address != base_ah {
                return Ok(ValidateCallbackResult::Invalid("PlayerUpdates link target is not an update of the base".into()));
             }
             // Check target entry is also a Player
             let _updated_player: Player = extract_entry_as(Some(target_record), "Updated Player")?;
         }
         _ => return Ok(ValidateCallbackResult::Invalid("PlayerUpdates link target is not an Update action".into()))
     }

     // 4. Author Check: Must be the player
     if create_link.author != original_player.player_key {
         return Ok(ValidateCallbackResult::Invalid("Author of PlayerUpdates link must be the player".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_to_scores_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be an AgentPubKey
     let base_agent = create_link.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerToScores base must be an AgentPubKey".into())))?;

    // 2. Target Check: Must be ActionHash of Score create
    let target_score_ah = create_link.target_address.clone().into_action_hash()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerToScores target must be a Score ActionHash".into())))?;
    let target_score_record = get(target_score_ah.clone(), GetOptions::default())?;
    let score: Score = extract_entry_as(target_score_record, "Score")?;

    // 3. Relationship Check: Score's player must match link base
     if score.player != base_agent {
         return Ok(ValidateCallbackResult::Invalid("Score's player does not match link base AgentPubKey".into()));
     }

    // 4. Author Check: Must be the player whose score it is
    if create_link.author != base_agent {
        return Ok(ValidateCallbackResult::Invalid("Author must be the Player whose score it is".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_presence_link(create_link: &CreateLink) -> ExternResult<ValidateCallbackResult> {
    // 1. Base Check: Must be the "presence" anchor hash
    let expected_base_hash = utils::anchor_for("presence")?; // Type AnyLinkableHash (EntryHash)
    if create_link.base_address != expected_base_hash {
        return Ok(ValidateCallbackResult::Invalid("Base for Presence link must be 'presence' anchor hash".into()));
    }

    // 2. Target Check: Must be ActionHash of Presence create
    let target_ah = create_link.target_address.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("Presence link target must be an ActionHash".into())))?;
    let target_record = get(target_ah.clone(), GetOptions::default())?;
    let presence: Presence = extract_entry_as(target_record, "Presence")?;

    // 3. Relationship Check: Optional - Presence agent matches action author? Usually true.
    let presence_action = must_get_record(target_ah)?.action().clone();
    if presence.agent_pubkey != *presence_action.author() {
        // This shouldn't happen if presence validation is correct, but check anyway
         warn!("Presence entry agent key does not match presence action author");
    }

    // 4. Author Check: Must be the agent whose presence it is
    if create_link.author != presence.agent_pubkey {
         return Ok(ValidateCallbackResult::Invalid("Author of Presence link must be the agent reporting presence".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}