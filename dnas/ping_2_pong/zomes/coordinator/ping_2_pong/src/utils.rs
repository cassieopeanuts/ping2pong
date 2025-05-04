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
        GetLinksInputBuilder::try_new(games_anchor, LinkTypes::GameIdToGame)?
            .build(),
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
        GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::PlayerToPlayers)?
        .build(),
    )?;
    Ok(!links.is_empty())
}

// Helper function to check if a player is already in an *InProgress* game.
pub fn is_player_in_ongoing_game(player_pub_key: &AgentPubKey) -> ExternResult<bool> {
    // Check games where the player is player1.
    let player1_links = get_links(
        GetLinksInputBuilder::try_new(player_pub_key.clone(), LinkTypes::Player1ToGames)?
            .build(),
    )?;

    for link in player1_links {
        if let Some(game_action_hash) = link.target.into_action_hash() {
            // Fetch the LATEST state of the game
            let maybe_record = crate::game::get_latest_game(game_action_hash)?;
            if let Some(record) = maybe_record {
                if let Some(entry_data) = record.entry().as_option() {
                     if let Ok(game) = Game::try_from(entry_data.clone()) {
                         // *** FIX: Only return true if the game status is InProgress ***
                         if game.game_status == GameStatus::InProgress {
                            debug!("Player {:?} found in InProgress game {:?} as Player 1", player_pub_key, record.action_hashed().hash);
                            return Ok(true);
                        }
                     } else { warn!("Failed to deserialize Game entry for record: {:?}", record.action_hashed().hash); }
                } else { warn!("Game record has no entry data: {:?}", record.action_hashed().hash); }
            }
        }
    }

    // Check games where the player is player2.
    let player2_links = get_links(
        GetLinksInputBuilder::try_new(player_pub_key.clone(), LinkTypes::Player2ToGames)?
            .build(),
    )?;

    for link in player2_links {
         if let Some(game_action_hash) = link.target.into_action_hash() {
             // Fetch the LATEST state of the game
             let maybe_record = crate::game::get_latest_game(game_action_hash)?;
             if let Some(record) = maybe_record {
                 if let Some(entry_data) = record.entry().as_option() {
                      if let Ok(game) = Game::try_from(entry_data.clone()) {
                          // *** FIX: Only return true if the game status is InProgress ***
                          if game.game_status == GameStatus::InProgress {
                             debug!("Player {:?} found in InProgress game {:?} as Player 2", player_pub_key, record.action_hashed().hash);
                             return Ok(true);
                         }
                      } else { warn!("Failed to deserialize Game entry for record: {:?}", record.action_hashed().hash); }
                 } else { warn!("Game record has no entry data: {:?}", record.action_hashed().hash); }
             }
         }
    }

    // If no InProgress games were found for the player
    debug!("Player {:?} not found in any InProgress game.", player_pub_key);
    Ok(false)
}