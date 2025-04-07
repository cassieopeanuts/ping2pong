// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/game.rs
use hdk::prelude::*;
use ping_2_pong_integrity::*;
// Use GameStatus directly from integrity crate
use ping_2_pong_integrity::game::GameStatus;
// Remove get_game_hash_by_id from utils import
use crate::utils::{ player_exists, is_player_in_ongoing_game, anchor_for }; // Import helpers
// Import Signal enum from lib.rs
use crate::Signal;

// Helper function to get all games. (No changes needed)
#[hdk_extern]
pub fn get_all_games(_: ()) -> ExternResult<Vec<Record>> {
    let games_anchor = anchor_for("games")?;
    let get_links_input = GetLinksInputBuilder::try_new(games_anchor, LinkTypes::GameIdToGame)?
        .build();
    let links = get_links(get_links_input)?;
    let get_inputs: Vec<GetInput> = links
        .into_iter()
        .filter_map(|link| {
            // Target of GameIdToGame is ActionHash
            link.target.into_action_hash().map(|ah| GetInput::new(ah.into(), GetOptions::default()))
        })
        .collect();

    if get_inputs.is_empty() {
        return Ok(vec![]); // Return empty vec if no games found
    }

    // Assuming HDK::with is correct for batch get in 0.4.1
    let records_result = HDK.with(|hdk| hdk.borrow().get(get_inputs));
    let records = match records_result {
      Ok(records) => records,
      Err(e) => return Err(wasm_error!(WasmErrorInner::Guest(format!("Failed to get game records: {:?}", e))))
    };
    Ok(records.into_iter().flatten().collect())
}

// get_game_hash_by_id is now in utils.rs
// player_exists is now in utils.rs
// is_player_in_ongoing_game is now in utils.rs


// Define PlayerStatus enum for lobby
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerStatus {
    Available,
    InGame,
}

// Function called by UI to check player status for lobby
#[hdk_extern]
pub fn get_player_status(player_pub_key: AgentPubKey) -> ExternResult<PlayerStatus> {
    if is_player_in_ongoing_game(&player_pub_key)? {
        Ok(PlayerStatus::InGame)
    } else {
        Ok(PlayerStatus::Available)
    }
}


#[hdk_extern]
pub fn create_game(mut game: Game) -> ExternResult<Record> {
    let my_pub_key = agent_info()?.agent_latest_pubkey;
    if game.player_1 != my_pub_key {
         if game.player_2.as_ref() != Some(&my_pub_key) {
              return Err(wasm_error!(WasmErrorInner::Guest("Game creator must be one of the players".into())));
         }
    }

    // Set required fields server-side
    game.created_at = sys_time()?;
    game.game_status = GameStatus::Waiting; // Use imported GameStatus
    game.player_1_paddle = 250;
    game.player_2_paddle = 250;
    game.ball_x = 400;
    game.ball_y = 300;

    // --- Perform Validations BEFORE creating entry ---
    if !player_exists(&game.player_1)? {
        return Err(wasm_error!(WasmErrorInner::Guest("Player 1 is not a registered player".into())));
    }
    if let Some(player2) = &game.player_2 {
        if !player_exists(player2)? {
            return Err(wasm_error!(WasmErrorInner::Guest("Player 2 is not a registered player".into())));
        }
        if game.player_1 == *player2 {
            return Err(wasm_error!(WasmErrorInner::Guest("Player 1 and Player 2 cannot be the same agent".into())));
        }
        if is_player_in_ongoing_game(player2)? {
            return Err(wasm_error!(WasmErrorInner::Guest("Player 2 is already in an ongoing game".into())));
        }
    }
    if is_player_in_ongoing_game(&game.player_1)? {
        return Err(wasm_error!(WasmErrorInner::Guest("Player 1 is already in an ongoing game".into())));
    }
    // --- End Validations ---

    // Create the Game entry.
    let game_action_hash = create_entry(&EntryTypes::Game(game.clone()))?;

    // Link from player1 pubkey to game action hash
    create_link(
        game.player_1.clone(),
        game_action_hash.clone(),
        LinkTypes::Player1ToGames,
        (),
    )?;

    // Link from player2 pubkey if exists
    if let Some(player2) = game.player_2.clone() {
        create_link(
            player2,
            game_action_hash.clone(),
            LinkTypes::Player2ToGames,
            (),
        )?;
    }

    // Link from global "games" anchor hash to game action hash
    let games_anchor_hash = anchor_for("games")?; // This is AnyLinkableHash(EntryHash)
    create_link(
        games_anchor_hash,
        game_action_hash.clone(), // Target must be AnyLinkableHash(ActionHash)
        LinkTypes::GameIdToGame,
        (),
    )?;

    // Fetch and return the created record
    let record = get(game_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find the newly created Game".to_string())))?;

    Ok(record)
}

// get_latest_game (No changes needed)
#[hdk_extern]
pub fn get_latest_game(original_game_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(original_game_hash.clone(), LinkTypes::GameUpdates)?.build(),
    )?;
    let latest_link = links
        .into_iter()
        .max_by(|link_a, link_b| link_a.timestamp.cmp(&link_b.timestamp));
    let latest_game_hash = match latest_link {
        Some(link) => {
            link.target
                .clone()
                .into_action_hash()
                .ok_or(wasm_error!(WasmErrorInner::Guest("No action hash associated with link".to_string())))?
        }
        None => original_game_hash.clone(),
    };
    get(latest_game_hash, GetOptions::default())
}

// get_original_game (No changes needed)
#[hdk_extern]
pub fn get_original_game(original_game_hash: ActionHash) -> ExternResult<Option<Record>> {
     let Some(details) = get_details(original_game_hash, GetOptions::default())? else {
        return Ok(None);
    };
    match details {
        Details::Record(details) => Ok(Some(details.record)),
        _ => Err(wasm_error!(WasmErrorInner::Guest("Malformed get details response".to_string()))),
    }
}

// get_all_revisions_for_game (No changes needed)
#[hdk_extern]
pub fn get_all_revisions_for_game(original_game_hash: ActionHash) -> ExternResult<Vec<Record>> {
     let Some(original_record) = get_original_game(original_game_hash.clone())? else {
        return Ok(vec![]);
    };
    let links = get_links(
        GetLinksInputBuilder::try_new(original_game_hash.clone(), LinkTypes::GameUpdates)?.build(),
    )?;
    let get_input: Vec<GetInput> = links
        .into_iter()
        .map(|link| {
            Ok(GetInput::new(
                link.target
                    .into_action_hash()
                    .ok_or(wasm_error!(WasmErrorInner::Guest("No action hash associated with link".to_string())))?
                    .into(),
                GetOptions::default(),
            ))
        })
        .collect::<ExternResult<Vec<GetInput>>>()?;

    if get_input.is_empty() {
        return Ok(vec![original_record]);
    }

    // Assuming HDK::with works for batch get
    let records_result = HDK.with(|hdk| hdk.borrow().get(get_input));
    let records = match records_result {
      Ok(records) => records,
      Err(e) => return Err(wasm_error!(WasmErrorInner::Guest(format!("Failed to get records: {:?}", e))))
    };

    let mut revision_records: Vec<Record> = records.into_iter().flatten().collect();
    revision_records.insert(0, original_record);
    Ok(revision_records)
}

// UpdateGameInput (No changes needed)
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateGameInput {
    pub original_game_hash: ActionHash,
    pub previous_game_hash: ActionHash,
    pub updated_game: Game,
}

// update_game (No changes needed)
#[hdk_extern]
pub fn update_game(input: UpdateGameInput) -> ExternResult<Record> {
    let updated_game_hash = update_entry(input.previous_game_hash.clone(), &input.updated_game)?;
    create_link(
        input.original_game_hash.clone(),
        updated_game_hash.clone(),
        LinkTypes::GameUpdates,
        (),
    )?;
    let record = get(updated_game_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly updated Game".to_string())
    ))?;
    Ok(record)
}

// delete_game (No significant changes needed)
#[hdk_extern]
pub fn delete_game(original_game_hash: ActionHash) -> ExternResult<ActionHash> {
    let details = get_details(original_game_hash.clone(), GetOptions::default())?.ok_or(
        wasm_error!(WasmErrorInner::Guest("Game not found".to_string())),
    )?;
    let record = match details {
        Details::Record(details) => details.record,
        _ => return Err(wasm_error!(WasmErrorInner::Guest("Malformed get details response".to_string()))),
    };
    let entry_option = record.entry().as_option(); // Use as_option()
    let entry = entry_option.ok_or(wasm_error!(WasmErrorInner::Guest("Game record has no entry".to_string())))?;
    let game = <Game>::try_from(entry.clone())?; // Clone entry for try_from

    // Delete player links
    let links1 = get_links(
        GetLinksInputBuilder::try_new(game.player_1.clone(), LinkTypes::Player1ToGames)?.build(),
    )?;
    for link in links1 {
        if let Some(action_hash) = link.target.into_action_hash() {
            if action_hash == original_game_hash {
                delete_link(link.create_link_hash)?;
            }
        }
    }
    if let Some(player2) = game.player_2 {
        let links2 = get_links(
            GetLinksInputBuilder::try_new(player2, LinkTypes::Player2ToGames)?.build(),
        )?;
        for link in links2 {
            if let Some(action_hash) = link.target.into_action_hash() {
                if action_hash == original_game_hash {
                    delete_link(link.create_link_hash)?;
                }
            }
        }
    }

    // Delete GameIdToGame link from anchor
    let games_anchor_hash = anchor_for("games")?;
    let anchor_links = get_links(
        GetLinksInputBuilder::try_new(games_anchor_hash, LinkTypes::GameIdToGame)? // Use the anchor hash as base
            .build(),
    )?;
     for link in anchor_links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if action_hash == original_game_hash {
                delete_link(link.create_link_hash)?;
            }
        }
    }

    // Delete the game entry itself
    delete_entry(original_game_hash)
}


// --- Presence and Invitation Logic ---


#[hdk_extern]
pub fn publish_presence(_: ()) -> ExternResult<ActionHash> {
    let agent = agent_info()?.agent_latest_pubkey;
    let now = sys_time()?.as_millis(); // now is i64

    // FIX: Convert i64 `now` to u64 for Presence struct using try_into()
    let timestamp_u64 = now.try_into().map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp conversion error (i64 to u64): {}", e))))?;

    let presence = Presence {
        agent_pubkey: agent,
        timestamp: timestamp_u64, // Assign the converted u64 timestamp
    };
    // Create entry first
    let presence_action_hash = create_entry(&EntryTypes::Presence(presence.clone()))?;

    // Link from global "presence" anchor hash to the presence action hash
    let presence_anchor_hash = anchor_for("presence")?;
    create_link(
        presence_anchor_hash,
        presence_action_hash.clone(),
        LinkTypes::Presence,
        (),
    )?;
    Ok(presence_action_hash)
}

#[hdk_extern]
pub fn get_online_users(_: ()) -> ExternResult<Vec<AgentPubKey>> {
    let presence_anchor_hash = anchor_for("presence")?;
    let links = get_links(
        GetLinksInputBuilder::try_new(presence_anchor_hash, LinkTypes::Presence)? // Use anchor hash
            .build(),
    )?;
    let mut online_agents: Vec<AgentPubKey> = Vec::new();
    let now_ms = sys_time()?.as_millis(); // i64
    let cutoff = now_ms.saturating_sub(30_000); // 30 second cutoff, cutoff is i64

    let get_inputs: Vec<GetInput> = links
        .into_iter()
        .filter_map(|link| link.target.into_action_hash()) // Get action hash targets
        .map(|ah| GetInput::new(ah.into(), GetOptions::default()))
        .collect();

     if get_inputs.is_empty() {
        return Ok(vec![]);
    }

    let records_result = HDK.with(|hdk| hdk.borrow().get(get_inputs));
    let records = match records_result {
      Ok(records) => records,
      Err(e) => return Err(wasm_error!(WasmErrorInner::Guest(format!("Failed to get presence records: {:?}", e))))
    };

    for record_option in records {
        if let Some(record) = record_option {
             // Use as_option() and clone entry data
             if let Some(entry_data) = record.entry().as_option() {
                 if let Ok(presence) = Presence::try_from(entry_data.clone()) {
                     // Compare i64 timestamp with i64 cutoff
                     if presence.timestamp >= cutoff.try_into().unwrap() {
                         if !online_agents.contains(&presence.agent_pubkey) {
                             online_agents.push(presence.agent_pubkey);
                         }
                     }
                 } else {
                     // Log error if entry is not Presence type
                      warn!("Failed to deserialize Presence entry for record: {:?}", record.action_hashed().hash);
                 }
             } else {
                // Log warning if record has no entry
                 warn!("Presence record has no entry data: {:?}", record.action_hashed().hash);
             }
        }
    }
    Ok(online_agents)
}

// send_invitation (No changes needed)
#[hdk_extern]
pub fn send_invitation(invitation: Invitation) -> ExternResult<()> {
    emit_signal(Signal::GameInvitation { // Use imported Signal
        game_id: invitation.game_id,
        inviter: invitation.inviter,
        message: invitation.message,
    })?;
    Ok(())
}

// Invitation struct (No changes needed)
#[derive(Serialize, Deserialize, Debug, Clone, SerializedBytes)]
pub struct Invitation {
    pub game_id: ActionHash,
    pub inviter: AgentPubKey,
    pub message: String,
}

// --- Other CRUD functions --- (Keep as is)
#[hdk_extern]
pub fn get_all_deletes_for_game(original_game_hash: ActionHash) -> ExternResult<Option<Vec<SignedActionHashed>>> {
    let Some(details) = get_details(original_game_hash, GetOptions::default())? else { return Ok(None); };
    match details {
        Details::Record(record_details) => Ok(Some(record_details.deletes)),
        _ => Err(wasm_error!(WasmErrorInner::Guest("Malformed details".into()))),
    }
}
#[hdk_extern]
pub fn get_oldest_delete_for_game(original_game_hash: ActionHash) -> ExternResult<Option<SignedActionHashed>> {
    let Some(mut deletes) = get_all_deletes_for_game(original_game_hash)? else { return Ok(None); };
    deletes.sort_by(|a, b| a.action().timestamp().cmp(&b.action().timestamp()));
    Ok(deletes.first().cloned())
}
#[hdk_extern]
pub fn get_games_for_player_1(player_1: AgentPubKey) -> ExternResult<Vec<Link>> {
    get_links(GetLinksInputBuilder::try_new(player_1, LinkTypes::Player1ToGames)?.build())
}
#[hdk_extern]
pub fn get_deleted_games_for_player_1(player_1: AgentPubKey) -> ExternResult<Vec<(SignedActionHashed, Vec<SignedActionHashed>)>> {
    let details = get_link_details(player_1, LinkTypes::Player1ToGames, None, GetOptions::default())?;
    Ok(details.into_inner().into_iter().filter(|(_, deletes)| !deletes.is_empty()).collect())
}
#[hdk_extern]
pub fn get_games_for_player_2(player_2: AgentPubKey) -> ExternResult<Vec<Link>> {
    get_links(GetLinksInputBuilder::try_new(player_2, LinkTypes::Player2ToGames)?.build())
}
#[hdk_extern]
pub fn get_deleted_games_for_player_2(player_2: AgentPubKey) -> ExternResult<Vec<(SignedActionHashed, Vec<SignedActionHashed>)>> {
    let details = get_link_details(player_2, LinkTypes::Player2ToGames, None, GetOptions::default())?;
    Ok(details.into_inner().into_iter().filter(|(_, deletes)| !deletes.is_empty()).collect())
}