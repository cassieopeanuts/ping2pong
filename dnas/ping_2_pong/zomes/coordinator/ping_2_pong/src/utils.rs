// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/utils.rs
use hdk::prelude::*;
use ping_2_pong_integrity::{LinkTypes, Game}; // Assuming integrity crate provides these
use ping_2_pong_integrity::game::GameStatus;
// No separate import for anchor_for needed here

// Public function definition within this module (crate::utils)
// This function delegates the call to the actual implementation in the integrity crate.
pub fn anchor_for(input: &str) -> ExternResult<AnyLinkableHash> {
    // Use the full path to call the function in the integrity crate's utils module
    ping_2_pong_integrity::utils::anchor_for(input)
}


// Helper function to get game hash by game_id (original ActionHash of the game entry).
pub fn get_game_hash_by_id(game_id: &ActionHash) -> ExternResult<Option<ActionHash>> {
    // Now uses the local `anchor_for` which delegates
    let games_anchor = anchor_for("games")?;
    let links = get_links(
        LinkQuery::try_new(games_anchor, LinkTypes::GameIdToGame)?,
        GetStrategy::default(),
    )?;

    for link in links {
        if let Some(target_hash) = link.target.into_action_hash() {
            if &target_hash == game_id {
                return Ok(Some(target_hash));
            }
        }
    }
    Ok(None)
}


// Helper function to check if a player exists (based on Player entry linked from AgentPubKey).
pub fn player_exists(agent_pub_key: &AgentPubKey) -> ExternResult<bool> {
    let links = get_links(
        LinkQuery::try_new(agent_pub_key.clone(), LinkTypes::PlayerToPlayers)?,
        GetStrategy::default(),
    )?;
    Ok(!links.is_empty())
}

// Helper function to check if a player is already in an *InProgress* game.
pub fn is_player_in_ongoing_game(player_pub_key: &AgentPubKey) -> ExternResult<bool> {
    debug!("[utils.rs] is_player_in_ongoing_game: Called for player: {:?}", player_pub_key);

    // Check games where the player is player1.
    let player1_links = get_links(
        LinkQuery::try_new(player_pub_key.clone(), LinkTypes::Player1ToGames)?,
        GetStrategy::default(),
    )?;

    for link in player1_links {
        if let Some(game_action_hash) = link.target.into_action_hash() {
            debug!("[utils.rs] is_player_in_ongoing_game: P1 Loop - Found game link for player {:?}: game_action_hash {:?}", player_pub_key, game_action_hash);
            // Fetch the LATEST state of the game
            let maybe_record = crate::game::get_latest_game(game_action_hash.clone())?; // Cloned game_action_hash
            debug!("[utils.rs] is_player_in_ongoing_game: P1 Loop - get_latest_game result for game {:?}: {:?}", game_action_hash, maybe_record.as_ref().map(|r| r.action_hashed().hash.clone()));
            if let Some(record) = maybe_record {
                if let Some(entry_data) = record.entry().as_option() {
                     if let Ok(game) = Game::try_from(entry_data.clone()) { // Assuming Game from ping_2_pong_integrity
                         debug!("[utils.rs] is_player_in_ongoing_game: P1 Loop - Game {:?} deserialized. Status: {:?}, P1: {:?}, P2: {:?}", game_action_hash, game.game_status, game.player_1, game.player_2.is_some());
                         // *** FIX: Only return true if the game status is InProgress ***
                         if game.game_status == GameStatus::InProgress { // Assuming GameStatus from ping_2_pong_integrity::game
                            debug!("[utils.rs] is_player_in_ongoing_game: P1 Loop - Player {:?} IS in InProgress game {:?}. Returning true.", player_pub_key, game_action_hash);
                            return Ok(true);
                        }
                     } else { warn!("[utils.rs] is_player_in_ongoing_game: P1 Loop - Failed to deserialize Game entry for record: {:?}", record.action_hashed().hash); }
                } else { warn!("[utils.rs] is_player_in_ongoing_game: P1 Loop - Game record has no entry data: {:?}", record.action_hashed().hash); }
            }
        }
    }

    // Check games where the player is player2.
    let player2_links = get_links(
        LinkQuery::try_new(player_pub_key.clone(), LinkTypes::Player2ToGames)?,
        GetStrategy::default(),
    )?;

    for link in player2_links {
         if let Some(game_action_hash) = link.target.into_action_hash() {
            debug!("[utils.rs] is_player_in_ongoing_game: P2 Loop - Found game link for player {:?}: game_action_hash {:?}", player_pub_key, game_action_hash);
             // Fetch the LATEST state of the game
             let maybe_record = crate::game::get_latest_game(game_action_hash.clone())?; // Cloned game_action_hash
             debug!("[utils.rs] is_player_in_ongoing_game: P2 Loop - get_latest_game result for game {:?}: {:?}", game_action_hash, maybe_record.as_ref().map(|r| r.action_hashed().hash.clone()));
             if let Some(record) = maybe_record {
                 if let Some(entry_data) = record.entry().as_option() {
                      if let Ok(game) = Game::try_from(entry_data.clone()) { // Assuming Game from ping_2_pong_integrity
                          debug!("[utils.rs] is_player_in_ongoing_game: P2 Loop - Game {:?} deserialized. Status: {:?}, P1: {:?}, P2: {:?}", game_action_hash, game.game_status, game.player_1, game.player_2.is_some());
                          // *** FIX: Only return true if the game status is InProgress ***
                          if game.game_status == GameStatus::InProgress { // Assuming GameStatus from ping_2_pong_integrity::game
                             debug!("[utils.rs] is_player_in_ongoing_game: P2 Loop - Player {:?} IS in InProgress game {:?}. Returning true.", player_pub_key, game_action_hash);
                             return Ok(true);
                         }
                      } else { warn!("[utils.rs] is_player_in_ongoing_game: P2 Loop - Failed to deserialize Game entry for record: {:?}", record.action_hashed().hash); }
                 } else { warn!("[utils.rs] is_player_in_ongoing_game: P2 Loop - Game record has no entry data: {:?}", record.action_hashed().hash); }
             }
         }
    }

    // If no InProgress games were found for the player
    debug!("[utils.rs] is_player_in_ongoing_game: Player {:?} is NOT in any InProgress game after checking all links. Returning false.", player_pub_key);
    Ok(false)
}