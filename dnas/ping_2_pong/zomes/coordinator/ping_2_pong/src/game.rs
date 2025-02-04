use hdk::prelude::*;
use ping_2_pong_integrity::*;
use crate::Signal;

// Helper function to get game hash
pub fn get_game_hash_by_id(game_id: &ActionHash) -> ExternResult<Option<ActionHash>> {
    // Use the game_id (an ActionHash) as the base of the link
    let links = get_links(
        GetLinksInputBuilder::try_new(game_id.clone(), LinkTypes::GameIdToGame)?
            .build(),
    )?;
    if let Some(link) = links.first() {
        let game_hash = link.target.clone().into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid game hash".to_string())),
        )?;
        Ok(Some(game_hash))
    } else {
        Ok(None)
    }
}

// Helper function to check if a player exists
fn player_exists(agent_pub_key: &AgentPubKey) -> ExternResult<bool> {
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_pub_key.clone(), LinkTypes::PlayerToPlayers)?.build(),
    )?;
    Ok(!links.is_empty())
}

// Helper function to check if a player is already in an ongoing game
fn is_player_in_ongoing_game(player_pub_key: &AgentPubKey) -> ExternResult<bool> {
    // Fetch all games where the player is player_1
    let player1_games = get_links(
        GetLinksInputBuilder::try_new(player_pub_key.clone(), LinkTypes::Player1ToGames)?.build(),
    )?;
    for link in player1_games {
        let game_hash = link.target.into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid game hash".to_string())),
        )?;
        let game_record = get(game_hash, GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found".to_string())))?;
        if let Some(game) = game_record
            .entry()
            .to_app_option::<Game>()
            .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        {
            if game.game_status == GameStatus::InProgress {
                return Ok(true);
            }
        }
    }
    // Fetch all games where the player is player_2
    let player2_games = get_links(
        GetLinksInputBuilder::try_new(player_pub_key.clone(), LinkTypes::Player2ToGames)?.build(),
    )?;
    for link in player2_games {
        let game_hash = link.target.into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid game hash".to_string())),
        )?;
        let game_record = get(game_hash, GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found".to_string())))?;
        if let Some(game) = game_record
            .entry()
            .to_app_option::<Game>()
            .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        {
            if game.game_status == GameStatus::InProgress {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[hdk_extern]
pub fn create_game(game: Game) -> ExternResult<Record> {
    // Create the Game entry. The integrity zome's validation callback will run automatically.
    let game_hash = create_entry(&EntryTypes::Game(game.clone()))?;

    // Link player_1 to the Game
    create_link(
        game.player_1.clone(),
        game_hash.clone(),
        LinkTypes::Player1ToGames,
        (),
    )?;

    // Link player_2 to the Game
    create_link(
        game.player_2.clone(),
        game_hash.clone(),
        LinkTypes::Player2ToGames,
        (),
    )?;

    // Create a link from game_id to the Game entry for efficient lookup
    create_link(
        game.game_id.clone(), // game_id is an ActionHash; use as is
        game_hash.clone(),
        LinkTypes::GameIdToGame,
        (),
    )?;

    let games_anchor = anchor_for("games")?;
    create_link(
        games_anchor,
        game_hash.clone(),
        LinkTypes::GameIdToGame, 
        (),
    )?;

    // Retrieve the created Game record
    let record = get(game_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find the newly created Game".to_string())))?;

    // --- Coordinator-level checks begin here ---
    // Check for uniqueness of game_id using the GameUpdates link.
    let existing_games = get_links(
        GetLinksInputBuilder::try_new(game.game_id.clone(), LinkTypes::GameUpdates)?
            .build(),
    )?;
    for link in existing_games {
        let existing_game_hash = link.target.into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid game hash".to_string())),
        )?;
        let existing_game_record = get(existing_game_hash, GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found".to_string())))?;
        if let Some(existing_game) = existing_game_record
            .entry()
            .to_app_option::<Game>()
            .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        {
            if existing_game.game_id == game.game_id {
                return Err(wasm_error!(WasmErrorInner::Guest("Game ID must be unique".into())));
            }
        }
    }

    // Ensure player_1 and player_2 are valid registered players.
    if !player_exists(&game.player_1)? {
        return Err(wasm_error!(WasmErrorInner::Guest("Player 1 is not a registered player".into())));
    }
    if !player_exists(&game.player_2)? {
        return Err(wasm_error!(WasmErrorInner::Guest("Player 2 is not a registered player".into())));
    }

    // Ensure player_1 and player_2 are not the same.
    if game.player_1 == game.player_2 {
        return Err(wasm_error!(WasmErrorInner::Guest("Player 1 and Player 2 cannot be the same agent".into())));
    }

    // Ensure game_status is initially Waiting.
    if game.game_status != GameStatus::Waiting {
        return Err(wasm_error!(WasmErrorInner::Guest("Newly created games must have status 'Waiting'".into())));
    }

    // Ensure player_1 is not in another ongoing game.
    if is_player_in_ongoing_game(&game.player_1)? {
        return Err(wasm_error!(WasmErrorInner::Guest("Player 1 is already in an ongoing game".into())));
    }

    // Ensure player_2 is not in another ongoing game.
    if is_player_in_ongoing_game(&game.player_2)? {
        return Err(wasm_error!(WasmErrorInner::Guest("Player 2 is already in an ongoing game".into())));
    }
    // --- End of coordinator-level checks ---

    Ok(record)
}

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
    let records = HDK.with(|hdk| hdk.borrow().get(get_input))?;
    let mut records: Vec<Record> = records.into_iter().flatten().collect();
    records.insert(0, original_record);
    Ok(records)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateGameInput {
    pub original_game_hash: ActionHash,
    pub previous_game_hash: ActionHash,
    pub updated_game: Game,
}

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

#[hdk_extern]
pub fn delete_game(original_game_hash: ActionHash) -> ExternResult<ActionHash> {
    let details = get_details(original_game_hash.clone(), GetOptions::default())?.ok_or(
        wasm_error!(WasmErrorInner::Guest("Game not found".to_string())),
    )?;
    let record = match details {
        Details::Record(details) => Ok(details.record),
        _ => Err(wasm_error!(WasmErrorInner::Guest("Malformed get details response".to_string()))),
    }?;
    let entry = record
        .entry()
        .as_option()
        .ok_or(wasm_error!(WasmErrorInner::Guest("Game record has no entry".to_string())))?;
    let game = <Game>::try_from(entry)?;
    let links = get_links(
        GetLinksInputBuilder::try_new(game.player_1.clone(), LinkTypes::Player1ToGames)?.build(),
    )?;
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if action_hash == original_game_hash {
                delete_link(link.create_link_hash)?;
            }
        }
    }
    let links = get_links(
        GetLinksInputBuilder::try_new(game.player_2.clone(), LinkTypes::Player2ToGames)?.build(),
    )?;
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if action_hash == original_game_hash {
                delete_link(link.create_link_hash)?;
            }
        }
    }
    delete_entry(original_game_hash)
}

#[hdk_extern]
pub fn get_all_deletes_for_game(original_game_hash: ActionHash) -> ExternResult<Option<Vec<SignedActionHashed>>> {
    let Some(details) = get_details(original_game_hash, GetOptions::default())? else {
        return Ok(None);
    };
    match details {
        Details::Entry(_) => Err(wasm_error!(WasmErrorInner::Guest("Malformed details".into()))),
        Details::Record(record_details) => Ok(Some(record_details.deletes)),
    }
}

#[hdk_extern]
pub fn get_oldest_delete_for_game(original_game_hash: ActionHash) -> ExternResult<Option<SignedActionHashed>> {
    let Some(mut deletes) = get_all_deletes_for_game(original_game_hash)? else {
        return Ok(None);
    };
    deletes.sort_by(|delete_a, delete_b| {
        delete_a.action().timestamp().cmp(&delete_b.action().timestamp())
    });
    Ok(deletes.first().cloned())
}

#[hdk_extern]
pub fn get_games_for_player_1(player_1: AgentPubKey) -> ExternResult<Vec<Link>> {
    get_links(GetLinksInputBuilder::try_new(player_1, LinkTypes::Player1ToGames)?.build())
}

#[hdk_extern]
pub fn get_deleted_games_for_player_1(
    player_1: AgentPubKey,
) -> ExternResult<Vec<(SignedActionHashed, Vec<SignedActionHashed>)>> {
    let details = get_link_details(
        player_1,
        LinkTypes::Player1ToGames,
        None,
        GetOptions::default(),
    )?;
    Ok(details.into_inner().into_iter().filter(|(_link, deletes)| !deletes.is_empty()).collect())
}

#[hdk_extern]
pub fn get_games_for_player_2(player_2: AgentPubKey) -> ExternResult<Vec<Link>> {
    get_links(GetLinksInputBuilder::try_new(player_2, LinkTypes::Player2ToGames)?.build())
}

#[hdk_extern]
pub fn get_deleted_games_for_player_2(
    player_2: AgentPubKey,
) -> ExternResult<Vec<(SignedActionHashed, Vec<SignedActionHashed>)>> {
    let details = get_link_details(
        player_2,
        LinkTypes::Player2ToGames,
        None,
        GetOptions::default(),
    )?;
    Ok(details.into_inner().into_iter().filter(|(_link, deletes)| !deletes.is_empty()).collect())
}

// Actual game logic and signaling here
#[hdk_extern]
pub fn update_paddle_position(input: PaddleUpdateInput) -> ExternResult<Record> {
    let current_record = get(input.previous_game_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found".to_string())))?;
    
    let mut current_game: Game = current_record.entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("SerializedBytesError: {:?}", e))))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Malformed game record".to_string())))?;
    
    // Update only the fields we care about.
    current_game.player_1_paddle = input.updated_game.player_1_paddle;
    current_game.player_2_paddle = input.updated_game.player_2_paddle;
    current_game.ball_x = input.updated_game.ball_x;
    current_game.ball_y = input.updated_game.ball_y;
    
    let updated_game_hash = update_entry(input.previous_game_hash, &current_game)?;
    
    create_link(
        input.original_game_hash,
        updated_game_hash.clone(),
        LinkTypes::GameUpdates,
        (),
    )?;
    
    // Emit the signal with the updated positions.
    let signal = Signal::GameUpdate {
        game_id: current_game.game_id.clone(),
        paddle1: current_game.player_1_paddle,
        paddle2: current_game.player_2_paddle,
        ball_x: current_game.ball_x,
        ball_y: current_game.ball_y,
    };
    emit_signal(signal)?;
    
    let record = get(updated_game_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find the updated game".to_string())))?;
    Ok(record)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PaddleUpdateInput {
    pub original_game_hash: ActionHash,
    pub previous_game_hash: ActionHash,
    pub updated_game: GameUpdateData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameUpdateData {
    pub game_id: ActionHash,
    pub player_1_paddle: u32,
    pub player_2_paddle: u32,
    pub ball_x: u32,
    pub ball_y: u32,
}
