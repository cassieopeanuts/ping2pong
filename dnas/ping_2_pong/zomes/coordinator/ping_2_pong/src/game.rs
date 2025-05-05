use hdk::prelude::*;
use ping_2_pong_integrity::*;
// Use GameStatus directly from integrity crate
use ping_2_pong_integrity::game::GameStatus;
// Import helpers from local utils module
use crate::utils::{ player_exists, is_player_in_ongoing_game, anchor_for };
// Import Signal enum definition from local lib.rs
use crate::Signal;

// --- Extern Functions ---

/// Fetches all game records linked from the global "games" anchor.
#[hdk_extern]
pub fn get_all_games(_: ()) -> ExternResult<Vec<Record>> {
    let games_anchor = anchor_for("games")?;
    let get_links_input = GetLinksInputBuilder::try_new(games_anchor, LinkTypes::GameIdToGame)?
        .build();
    let links = get_links(get_links_input)?;

    // Prepare inputs for a batch get operation
    let get_inputs: Vec<GetInput> = links
        .into_iter()
        .filter_map(|link| {
            // The target of GameIdToGame link is the ActionHash of the game creation
            link.target.into_action_hash().map(|ah| GetInput::new(ah.into(), GetOptions::default()))
        })
        .collect();

    if get_inputs.is_empty() {
        return Ok(vec![]); // No games found
    }

    // Perform the batch get using HDK::with for efficiency
    // Note: Error handling wraps the result in a WasmError
    let records_result = HDK.with(|hdk| hdk.borrow().get(get_inputs));
    let records = match records_result {
      Ok(records) => records,
      Err(e) => return Err(wasm_error!(WasmErrorInner::Guest(format!("Failed to get game records: {:?}", e))))
    };

    // Flatten the results (get returns Vec<Option<Record>>) and collect valid records
    Ok(records.into_iter().flatten().collect())
}


/// Enum representing a player's status for the lobby.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlayerStatus {
    Available,
    InGame,
}

/// Checks if a player is currently involved in an 'InProgress' game.
#[hdk_extern]
pub fn get_player_status(player_pub_key: AgentPubKey) -> ExternResult<PlayerStatus> {
    if is_player_in_ongoing_game(&player_pub_key)? {
        Ok(PlayerStatus::InGame)
    } else {
        Ok(PlayerStatus::Available)
    }
}


/// Input structure for the `create_game` function.
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateGameInput {
    pub player_1: AgentPubKey,
    pub player_2: Option<AgentPubKey>, // Optional: Used for direct invitations
}


/// Allows a player (caller) to join an existing game that is in 'Waiting' status.
/// Updates the game status to 'InProgress' and emits a 'GameStarted' signal.
#[hdk_extern]
pub fn join_game(original_game_hash: ActionHash) -> ExternResult<Record> {
    let caller_pubkey = agent_info()?.agent_latest_pubkey; // This is Player 2 joining
    debug!("[join_game] Agent {:?} attempting to join game {:?}", caller_pubkey, original_game_hash);

    // 1. Get the latest state of the game record being joined
    let latest_game_record_option = get_latest_game(original_game_hash.clone())?;
    let latest_game_record = match latest_game_record_option {
        Some(record) => record,
        None => return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Cannot join game: Game record not found for original hash {:?}", original_game_hash
        )))),
    };

    // Extract current game state and the hash of the action we are updating
    let previous_action_hash = latest_game_record.action_hashed().hash.clone();
    let entry = latest_game_record.entry().as_option()
        .ok_or(wasm_error!(WasmErrorInner::Guest("Latest game record for join has no entry".to_string())))?
        .clone();
    let current_game = Game::try_from(entry)?;
    let player1_pubkey = current_game.player_1.clone(); // Store Player 1's key for signal sending

    // 2. Validate if joining is allowed
    if current_game.game_status != GameStatus::Waiting {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Cannot join game: Game status is not 'Waiting', it's {:?}", current_game.game_status
        ))));
    }
    // Check if Player 2 slot is already taken by someone else
    if current_game.player_2.is_some() && current_game.player_2.as_ref() != Some(&caller_pubkey) {
        return Err(wasm_error!(WasmErrorInner::Guest("Cannot join game: Player 2 slot is already taken by another player".into())));
    }
    // Player 1 cannot join their own game as Player 2
    if player1_pubkey == caller_pubkey {
        return Err(wasm_error!(WasmErrorInner::Guest("Cannot join game: Player 1 cannot join their own game as Player 2".into())));
    }
    // Check if the joining player is already in another active game
    if is_player_in_ongoing_game(&caller_pubkey)? {
         // Allow re-joining the *same* game if P2 was already set but status somehow remained Waiting
         if !(current_game.player_2.as_ref() == Some(&caller_pubkey) && current_game.game_status == GameStatus::Waiting) {
             return Err(wasm_error!(WasmErrorInner::Guest("Cannot join game: You are already in another ongoing game".into())));
         }
    }
    // Ensure the joining player has a profile
     if !player_exists(&caller_pubkey)? {
        return Err(wasm_error!(WasmErrorInner::Guest("Cannot join game: Joining player does not have a profile".into())));
     }


    // 3. Prepare the updated game state with Player 2 added and status changed
    let updated_game = Game {
        player_1: player1_pubkey.clone(),
        player_2: Some(caller_pubkey.clone()),
        game_status: GameStatus::InProgress, // Set status to InProgress
        created_at: current_game.created_at,
        player_1_paddle: current_game.player_1_paddle, // Keep default positions
        player_2_paddle: current_game.player_2_paddle,
        ball_x: current_game.ball_x,
        ball_y: current_game.ball_y,
    };

    // 4. Commit the update action to the DHT
    debug!("[join_game] Updating game entry {:?} to add player 2 and set status InProgress", previous_action_hash);
    let update_action_hash = update_entry(previous_action_hash.clone(), &updated_game)?;
    debug!("[join_game] Game entry updated with action hash {:?}", update_action_hash);

    // 5. Create the link: Player 2 -> Original Game Hash
    // This helps find games a player is involved in as Player 2.
    create_link(
        caller_pubkey.clone(),
        original_game_hash.clone(),
        LinkTypes::Player2ToGames,
        (),
    )?;
    debug!("[join_game] Created Player2ToGames link for agent {:?}", caller_pubkey);

    // 6. Create the GameUpdates link: Original Game Hash -> Update Action Hash
    // This allows tracking the history/revisions of a game.
     create_link(
        original_game_hash.clone(),
        update_action_hash.clone(),
        LinkTypes::GameUpdates,
        (),
    )?;
    debug!("[join_game] Created GameUpdates link from {:?} to {:?}", original_game_hash, update_action_hash);

    // 7. *** Emit GameStarted signal (Broadcast) ***
    //    This signal informs connected UIs that the game is ready to start.
    //    We broadcast because `remote_signal` isn't available/reliable in HDK 0.4.x.
    //    The UI (`App.svelte`) will filter and react only if the current user is P1 or P2.
    let start_sig = Signal::GameStarted {
         game_id: original_game_hash.clone(),
         player_1: player1_pubkey.clone(), // Include P1 pubkey
         player_2: caller_pubkey.clone(),   // Include P2 pubkey (the caller)
    };

    // 7. Broadcast locally (player 2) …
    emit_signal(&start_sig)?;

    // 8. Relay to player 1 – synchronous RPC
    call_remote(
        player1_pubkey.clone(),          // destination agent
        zome_info()?.name,               // current zome name
        "receive_remote_signal".into(),  // the helper you just added
        None,                            // provenance (cap secret)
        &start_sig                       // same payload
    )?;

    debug!("[join_game] Emitted GameStarted signal (broadcast): {:?}", start_sig);


    // 8. Fetch and return the latest record (representing the update action)
    //    This confirms the update and provides the latest state to the caller (Player 2).
    let final_record = get(update_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!(
            "Could not find the updated Game record after join: {:?}", update_action_hash
        ))))?;

    Ok(final_record)
}

/// Creates a new game entry, optionally specifying Player 2 for an invitation.
/// Links the game to players and the global games anchor.
#[hdk_extern]
pub fn create_game(input: CreateGameInput) -> ExternResult<Record> {
    let my_pub_key = agent_info()?.agent_latest_pubkey;
    debug!("[create_game] Agent {:?} creating game with input: {:?}", my_pub_key, input);

    // Validate creator is one of the players involved
    if input.player_1 != my_pub_key && input.player_2.as_ref() != Some(&my_pub_key) {
        return Err(wasm_error!(WasmErrorInner::Guest("Game creator must be Player 1 or specified Player 2".into())));
    }

    // --- Pre-creation Validations ---
    // Ensure Player 1 exists and isn't already in a game
    if !player_exists(&input.player_1)? {
        return Err(wasm_error!(WasmErrorInner::Guest("Player 1 is not a registered player".into())));
    }
    if is_player_in_ongoing_game(&input.player_1)? {
        return Err(wasm_error!(WasmErrorInner::Guest("Player 1 is already in an ongoing game".into())));
    }
    // If Player 2 is specified (for an invite), validate them too
    if let Some(player2) = &input.player_2 {
        if !player_exists(player2)? {
            return Err(wasm_error!(WasmErrorInner::Guest("Player 2 is not a registered player".into())));
        }
        if input.player_1 == *player2 {
            return Err(wasm_error!(WasmErrorInner::Guest("Player 1 and Player 2 cannot be the same agent".into())));
        }
        if is_player_in_ongoing_game(player2)? {
            return Err(wasm_error!(WasmErrorInner::Guest("Player 2 is already in an ongoing game".into())));
        }
    }
    // --- End Validations ---

    // Construct the initial Game entry state
    let game = Game {
        player_1: input.player_1.clone(),
        player_2: input.player_2.clone(), // None if not invited, Some(pubkey) if invited
        created_at: sys_time()?,          // Set creation timestamp
        game_status: GameStatus::Waiting, // Always start as Waiting
        player_1_paddle: 250,             // Default positions
        player_2_paddle: 250,
        ball_x: 400,
        ball_y: 300,
    };
    debug!("[create_game] Constructed game entry: {:?}", game);

    // Create the Game entry on the DHT
    let game_action_hash = create_entry(&EntryTypes::Game(game.clone()))?;
    debug!("[create_game] Game entry created with action hash: {:?}", game_action_hash);

    // --- Create Necessary Links ---
    // Link from Player 1 to the game
    create_link(game.player_1.clone(), game_action_hash.clone(), LinkTypes::Player1ToGames, (),)?;
    // Link from Player 2 to the game (only if Player 2 was specified)
    if let Some(player2) = game.player_2.clone() {
        create_link(player2, game_action_hash.clone(), LinkTypes::Player2ToGames, (),)?;
    }
    // Link from the global "games" anchor to the game (for discoverability)
    let games_anchor_hash = anchor_for("games")?;
    create_link(games_anchor_hash, game_action_hash.clone(), LinkTypes::GameIdToGame, (),)?;
    debug!("[create_game] Links created successfully.");

    // Fetch and return the created record
    let record = get(game_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find the newly created Game".to_string())))?;

    Ok(record)
}


/// Retrieves the latest version of a game record, following the GameUpdates links.
#[hdk_extern]
pub fn get_latest_game(original_game_hash: ActionHash) -> ExternResult<Option<Record>> {
    // Get links pointing away from the original game hash with the 'GameUpdates' type
    let links = get_links(
        GetLinksInputBuilder::try_new(original_game_hash.clone(), LinkTypes::GameUpdates)?.build(),
    )?;
    // Find the link with the latest timestamp
    let latest_link = links
        .into_iter()
        .max_by(|link_a, link_b| link_a.timestamp.cmp(&link_b.timestamp));

    // Determine the hash of the latest action (either the target of the latest link or the original hash)
    let latest_game_hash = match latest_link {
        Some(link) => {
            link.target
                .clone()
                .into_action_hash() // Target should be an ActionHash
                .ok_or(wasm_error!(WasmErrorInner::Guest("GameUpdates link target is not an ActionHash".to_string())))?
        }
        None => original_game_hash.clone(), // No updates found, use the original hash
    };
    // Get the record associated with the latest action hash
    get(latest_game_hash, GetOptions::default())
}

/// Retrieves the original record of a game creation action.
#[hdk_extern]
pub fn get_original_game(original_game_hash: ActionHash) -> ExternResult<Option<Record>> {
     let Some(details) = get_details(original_game_hash, GetOptions::default())? else {
        return Ok(None); // Not found
    };
    // Ensure the details fetched correspond to a Record
    match details {
        Details::Record(details) => Ok(Some(details.record)),
        _ => Err(wasm_error!(WasmErrorInner::Guest("Malformed get details response: Expected Record".to_string()))),
    }
}

/// Retrieves all historical records (revisions) for a given game, starting with the original.
#[hdk_extern]
pub fn get_all_revisions_for_game(original_game_hash: ActionHash) -> ExternResult<Vec<Record>> {
     // Get the original record first
     let Some(original_record) = get_original_game(original_game_hash.clone())? else {
        return Ok(vec![]); // Return empty if original doesn't exist
    };
    // Get all update links originating from the original game action
    let links = get_links(
        GetLinksInputBuilder::try_new(original_game_hash.clone(), LinkTypes::GameUpdates)?.build(),
    )?;
    // Prepare inputs for a batch get of all linked update actions
    let get_input: Vec<GetInput> = links
        .into_iter()
        .map(|link| {
            Ok(GetInput::new(
                link.target
                    .into_action_hash()
                    .ok_or(wasm_error!(WasmErrorInner::Guest("GameUpdates link target is not an ActionHash".to_string())))?
                    .into(), // Convert ActionHash to AnyLinkableHash for GetInput
                GetOptions::default(),
            ))
        })
        .collect::<ExternResult<Vec<GetInput>>>()?; // Collect results, handling potential errors

    // If no update links exist, return just the original record
    if get_input.is_empty() {
        return Ok(vec![original_record]);
    }

    // Perform the batch get for all update records
    let records_result = HDK.with(|hdk| hdk.borrow().get(get_input));
    let records = match records_result {
      Ok(records) => records,
      Err(e) => return Err(wasm_error!(WasmErrorInner::Guest(format!("Failed to get game revision records: {:?}", e))))
    };

    // Collect the valid revision records and prepend the original record
    let mut revision_records: Vec<Record> = records.into_iter().flatten().collect();
    revision_records.insert(0, original_record); // Add original at the beginning
    Ok(revision_records)
}

/// Input structure for the `update_game` function.
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateGameInput {
    pub original_game_hash: ActionHash, // Hash of the initial game creation action
    pub previous_game_hash: ActionHash, // Hash of the action being updated
    pub updated_game: Game,             // The new game state
}

/// Updates a game entry. Creates a new action and links it to the original game via GameUpdates.
#[hdk_extern]
pub fn update_game(input: UpdateGameInput) -> ExternResult<Record> {
    // Commit the update action, referencing the previous action hash
    let updated_game_hash = update_entry(input.previous_game_hash.clone(), &input.updated_game)?;
    // Create a link from the original game to this new update action
    create_link(
        input.original_game_hash.clone(),
        updated_game_hash.clone(),
        LinkTypes::GameUpdates,
        (),
    )?;
    // Fetch and return the newly created update record
    let record = get(updated_game_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly updated Game record".to_string())
    ))?;
    Ok(record)
}

/// Deletes a game entry and its associated links. Only allowed for games in 'Waiting' status.
#[hdk_extern]
pub fn delete_game(original_game_hash: ActionHash) -> ExternResult<ActionHash> {
    // Fetch the record to be deleted
    let details = get_details(original_game_hash.clone(), GetOptions::default())?.ok_or(
        wasm_error!(WasmErrorInner::Guest("Game not found".to_string())),
    )?;
    let record = match details {
        Details::Record(details) => details.record,
        _ => return Err(wasm_error!(WasmErrorInner::Guest("Malformed get details response: Expected Record".to_string()))),
    };
    // Deserialize the game entry
    let entry = record.entry().as_option().ok_or(wasm_error!(WasmErrorInner::Guest("Game record has no entry".to_string())))?.clone();
    let game = <Game>::try_from(entry)?;

    // --- Validation: Only allow deleting 'Waiting' games ---
    // This check prevents deleting games that are in progress or finished.
    if game.game_status != GameStatus::Waiting {
         return Err(wasm_error!(WasmErrorInner::Guest("Only games in 'Waiting' status can be deleted".to_string())));
    }

    // --- Link Deletion ---
    // Delete links from Player 1 to this game
    let links1 = get_links( GetLinksInputBuilder::try_new(game.player_1.clone(), LinkTypes::Player1ToGames)?.build(), )?;
    for link in links1 { if let Some(action_hash) = link.target.into_action_hash() { if action_hash == original_game_hash { delete_link(link.create_link_hash)?; } } }
    // Delete links from Player 2 (if exists) to this game
    if let Some(player2) = game.player_2 {
        let links2 = get_links( GetLinksInputBuilder::try_new(player2, LinkTypes::Player2ToGames)?.build(), )?;
        for link in links2 { if let Some(action_hash) = link.target.into_action_hash() { if action_hash == original_game_hash { delete_link(link.create_link_hash)?; } } }
    }
    // Delete link from the global "games" anchor to this game
    let games_anchor_hash = anchor_for("games")?;
    let anchor_links = get_links( GetLinksInputBuilder::try_new(games_anchor_hash, LinkTypes::GameIdToGame)?.build(), )?;
     for link in anchor_links { if let Some(action_hash) = link.target.into_action_hash() { if action_hash == original_game_hash { delete_link(link.create_link_hash)?; } } }

    // --- Entry Deletion ---
    // Delete the game entry itself
    delete_entry(original_game_hash)
}


// --- Presence and Invitation Logic ---

/// Creates a Presence entry and links it from the global "presence" anchor.
#[hdk_extern]
pub fn publish_presence(_: ()) -> ExternResult<ActionHash> {
    let agent = agent_info()?.agent_latest_pubkey;
    let now = sys_time()?.as_millis();
    // Convert timestamp safely
    let timestamp_u64 = now.try_into().map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Timestamp conversion error (i64 to u64): {}", e))))?;

    let presence = Presence { agent_pubkey: agent, timestamp: timestamp_u64 };
    // Create the entry
    let presence_action_hash = create_entry(&EntryTypes::Presence(presence.clone()))?;
    // Link from anchor
    let presence_anchor_hash = anchor_for("presence")?;
    create_link( presence_anchor_hash, presence_action_hash.clone(), LinkTypes::Presence, (), )?;
    Ok(presence_action_hash)
}

/// Retrieves a list of AgentPubKeys considered "online" based on recent Presence entries.
#[hdk_extern]
pub fn get_online_users(_: ()) -> ExternResult<Vec<AgentPubKey>> {
     let presence_anchor_hash = anchor_for("presence")?;
    // Get links from the presence anchor
    let links = get_links( GetLinksInputBuilder::try_new(presence_anchor_hash, LinkTypes::Presence)?.build(), )?;
    let mut online_agents: Vec<AgentPubKey> = Vec::new();
    let now_ms = sys_time()?.as_millis();
    let cutoff = now_ms.saturating_sub(30_000); // 30 second cutoff

    // Prepare batch get for presence entries
    let get_inputs: Vec<GetInput> = links .into_iter() .filter_map(|link| link.target.into_action_hash()) .map(|ah| GetInput::new(ah.into(), GetOptions::default())) .collect();
     if get_inputs.is_empty() { return Ok(vec![]); }

    // Fetch presence records
    let records_result = HDK.with(|hdk| hdk.borrow().get(get_inputs));
    let records = match records_result {
      Ok(records) => records,
      Err(e) => return Err(wasm_error!(WasmErrorInner::Guest(format!("Failed to get presence records: {:?}", e))))
    };

    // Process records to find recent ones
    for record_option in records {
        if let Some(record) = record_option {
             if let Some(entry_data) = record.entry().as_option() {
                 // Try to deserialize as Presence entry
                 if let Ok(presence) = Presence::try_from(entry_data.clone()) {
                     // Convert cutoff safely for comparison
                     let cutoff_u64 = u64::try_from(cutoff).unwrap_or(0);
                     // Check if timestamp is recent and agent not already added
                     if presence.timestamp >= cutoff_u64 {
                         if !online_agents.contains(&presence.agent_pubkey) {
                             online_agents.push(presence.agent_pubkey);
                         }
                     }
                 } else { warn!("Failed to deserialize Presence entry for record: {:?}", record.action_hashed().hash); }
             } else { warn!("Presence record has no entry data: {:?}", record.action_hashed().hash); }
        }
    }
    Ok(online_agents)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InvitationPayload {
    pub game_id : ActionHash,
    pub invitee : AgentPubKey,
    pub message : String,
}


/// Data structure for game invitations.
#[derive(Serialize, Deserialize, Debug, Clone, SerializedBytes)]
pub struct Invitation {
    pub game_id: ActionHash,
    pub inviter: AgentPubKey,
    pub message: String,
}

// --- Other CRUD functions ---

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
