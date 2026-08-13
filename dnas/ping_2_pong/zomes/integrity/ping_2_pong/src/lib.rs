use hdi::prelude::*;

fn custom_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    for byte in buf.iter_mut() {
        *byte = 0;
    }
    Ok(())
}
getrandom::register_custom_getrandom!(custom_getrandom);

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
pub mod chat;
pub use chat::ChatMessage;

// Import validation functions for entries
pub mod game_validation;
pub mod player_validation;
pub mod score_validation;
pub mod statistics_validation;
pub mod presence_validation;

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
    #[entry_type(visibility = "public")]
    ChatMessage(ChatMessage),
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
    AllPlayersAnchorToAgentPubKey, // For linking the "all_players" anchor to each player's AgentPubKey
    AllChatMessagesAnchorToMessage,
}


// Main Validation Callback
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::CreateEntry(OpEntry::CreateEntry { app_entry, action }) => match app_entry {
            EntryTypes::Game(game) => game_validation::validate_create_game(&action, game),
            EntryTypes::Player(player) => player_validation::validate_create_player(&action, player),
            EntryTypes::Score(score) => score_validation::validate_create_score(&action, score),
            EntryTypes::Statistics(statistics) => statistics_validation::validate_create_statistics(&action, statistics),
            EntryTypes::Presence(presence) => presence_validation::validate_create_presence(&action, presence),
            EntryTypes::AnchorPath(_) => Ok(ValidateCallbackResult::Valid),
            EntryTypes::ChatMessage(msg) => {
                if msg.content.trim().is_empty() {
                    return Ok(ValidateCallbackResult::Invalid("Chat message content cannot be empty".into()));
                }
                Ok(ValidateCallbackResult::Valid)
            }
        },
        FlatOp::Link(OpLink::CreateLink { link_type, action }) => match link_type {
            LinkTypes::GameIdToGame => validate_gameid_to_game_link(&action),
            LinkTypes::Player1ToGames => validate_player1_to_game_link(&action),
            LinkTypes::Player2ToGames => validate_player2_to_game_link(&action),
            LinkTypes::GameUpdates => validate_game_updates_link(&action),
            LinkTypes::GameToScores => validate_game_to_score_link(&action),
            LinkTypes::GameToStatistics => validate_game_to_statistics_link(&action),
            LinkTypes::PlayerToPlayers => validate_player_to_players_link(&action),
            LinkTypes::PlayerNameToPlayer => validate_playername_to_player_link(&action),
            LinkTypes::PlayerUpdates => validate_player_updates_link(&action),
            LinkTypes::PlayerToScores => validate_player_to_scores_link(&action),
            LinkTypes::Presence => validate_presence_link(&action),
            LinkTypes::AllPlayersAnchorToAgentPubKey => {
                if action.base_address.clone().into_entry_hash().is_none() {
                    return Ok(ValidateCallbackResult::Invalid("AllPlayersAnchorToAgentPubKey base must be an EntryHash (anchor)".into()));
                }
                if action.target_address.clone().into_agent_pub_key().is_none() {
                    return Ok(ValidateCallbackResult::Invalid("AllPlayersAnchorToAgentPubKey target must be an AgentPubKey".into()));
                }
                Ok(ValidateCallbackResult::Valid)
            }
            LinkTypes::AllChatMessagesAnchorToMessage => {
                if action.base_address.clone().into_entry_hash().is_none() {
                    return Ok(ValidateCallbackResult::Invalid("AllChatMessagesAnchorToMessage base must be an EntryHash (anchor)".into()));
                }
                if action.target_address.clone().into_action_hash().is_none() {
                    return Ok(ValidateCallbackResult::Invalid("AllChatMessagesAnchorToMessage target must be an ActionHash".into()));
                }
                Ok(ValidateCallbackResult::Valid)
            }
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

// --- Simplified Link Validations (No `get` calls inside) ---

fn validate_gameid_to_game_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
    let _base_hash: AnyLinkableHash = action.base_address.clone();
    if action.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("GameIdToGame target must be an ActionHash".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player1_to_game_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
    let base_agent = action.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("Player1ToGames base must be an AgentPubKey".into())))?;
    if action.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("Player1ToGames target must be an ActionHash".into()));
    }
    if action.author() != &base_agent {
         return Ok(ValidateCallbackResult::Invalid("Author of Player1ToGames link must be Player 1".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player2_to_game_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
    let _base_agent = action.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("Player2ToGames base must be an AgentPubKey".into())))?;
    if action.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("Player2ToGames target must be an ActionHash".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_updates_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
     if action.base_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("GameUpdates base must be an ActionHash".into()));
     }
     if action.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("GameUpdates target must be an ActionHash".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_to_score_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
     if action.base_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToScores base must be a Game ActionHash".into()));
     }
     if action.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToScores target must be a Score ActionHash".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_game_to_statistics_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
      if action.base_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToStatistics base must be a Game ActionHash".into()));
     }
     if action.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("GameToStatistics target must be a Statistics ActionHash".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_to_players_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
     let base_agent = action.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerToPlayers base must be an AgentPubKey".into())))?;
    if action.target_address.clone().into_action_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("PlayerToPlayers target must be a Player ActionHash".into()));
    }
    if action.author() != &base_agent {
        return Ok(ValidateCallbackResult::Invalid("Author must be the Player themselves".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_playername_to_player_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
     if action.base_address.clone().into_entry_hash().is_none() {
        return Ok(ValidateCallbackResult::Invalid("PlayerNameToPlayer base must be an EntryHash (Anchor)".into()));
     }
     if action.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerNameToPlayer target must be a Player ActionHash".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_updates_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
     if action.base_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerUpdates base must be an ActionHash".into()));
     }
     if action.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerUpdates target must be an ActionHash".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_player_to_scores_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
     let _base_agent = action.base_address.clone().into_agent_pub_key()
         .ok_or(wasm_error!(WasmErrorInner::Guest("PlayerToScores base must be an AgentPubKey".into())))?;
    if action.target_address.clone().into_action_hash().is_none() {
         return Ok(ValidateCallbackResult::Invalid("PlayerToScores target must be a Score ActionHash".into()));
     }
    Ok(ValidateCallbackResult::Valid)
}

fn validate_presence_link(action: &TypedAction<CreateLinkData>) -> ExternResult<ValidateCallbackResult> {
    if action.base_address.clone().into_entry_hash().is_none() && action.base_address.clone().into_agent_pub_key().is_none() {
         return Ok(ValidateCallbackResult::Invalid("Base for Presence link must be an EntryHash or AgentPubKey".into()));
    }
    Ok(ValidateCallbackResult::Valid)
}