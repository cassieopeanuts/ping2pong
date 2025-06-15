use hdk::prelude::*;
use ping_2_pong_integrity::*;
use crate::utils::get_game_hash_by_id; 
use ping_2_pong_integrity::game::GameStatus; 
use ping_2_pong_integrity::Game; 

// Maximum allowed score points.
const MAX_POINTS: u32 = 100; // Keep high for flexibility, game logic enforces 10

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateScoreInput {
    pub game_id: ActionHash,
    pub player: AgentPubKey,
    pub player_points: u32,
}

#[hdk_extern]
pub fn create_score(input: CreateScoreInput) -> ExternResult<Record> {
    debug!("[score.rs] create_score: Called with input: {:?}", input);

    // --- Validation ---
    // Ensure the game_id corresponds to an actual Game entry
    let game_action_hash = get_game_hash_by_id(&input.game_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!("Game ID does not exist: {}", input.game_id))))?;

    // Fetch the *latest* game state record to check status
    debug!("[score.rs] create_score: Fetching game record for game_id: {:?}", input.game_id);
    match crate::game::get_latest_game(game_action_hash.clone()) { 
        Ok(Some(game_record)) => {
            match game_record.entry().to_app_option::<ping_2_pong_integrity::Game>() {
                Ok(Some(game_entry)) => {
                    debug!("[score.rs] create_score: For game_id {:?}, current game status is: {:?}", input.game_id, game_entry.game_status);
                    if game_entry.game_status != ping_2_pong_integrity::game::GameStatus::Finished {
                        debug!("[score.rs] create_score: WARNING - Attempting to create score for a game (id: {:?}) not in 'Finished' state. Actual status: {:?}", input.game_id, game_entry.game_status);
                    }
                }
                Err(e) => debug!("[score.rs] create_score: Error deserializing game entry for game_id {:?}: {:?}", input.game_id, e),
                Ok(None) => debug!("[score.rs] create_score: Game entry data not found for game_id {:?}", input.game_id),
            }
        }
        Err(e) => debug!("[score.rs] create_score: Error fetching game record for game_id {:?}: {:?}", input.game_id, e),
        Ok(None) => debug!("[score.rs] create_score: No game record found for game_id {:?}", input.game_id),
    }

    let game_record_for_validation = crate::game::get_latest_game(game_action_hash.clone())? 
        .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found for validation".into())))?;
    let game_for_validation = game_record_for_validation
        .entry()
        .to_app_option::<Game>() 
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid Game entry format for validation".into())))?;

    // Ensure the game status is Finished before recording score
    if game_for_validation.game_status != GameStatus::Finished { 
        return Err(wasm_error!(WasmErrorInner::Guest("Scores can only be recorded for 'Finished' games".into())));
    }

    // Ensure the score is being assigned to a player who was actually in the game.
    if input.player != game_for_validation.player_1 && game_for_validation.player_2.as_ref() != Some(&input.player) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Score must be assigned to a player who participated in the game".into()
        )));
    }
    // Corrected the second instance of the check to use the correct variable names
    if input.player != game_for_validation.player_1 && game_for_validation.player_2.as_ref() != Some(&input.player) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Score must be assigned to a player who participated in the game (repeated check with correct vars)".into()
        )));
    }

    // Validate that the score points are within a reasonable range.
    if input.player_points > MAX_POINTS { 
        warn!("Score points {} exceed MAX_POINTS {}", input.player_points, MAX_POINTS);
        // Allow high scores for now, UI/game logic should enforce game rules like first to 10.
    }
     if input.player_points > 10 { // Add a more reasonable sanity check
         warn!("Recorded score {} seems high.", input.player_points);
     }
     // --- End Validation ---


    // Create the Score entry.
    let score_to_create = Score {
        game_id: input.game_id.clone(),
        player: input.player.clone(),
        player_points: input.player_points,
        created_at: sys_time()?,
    };
    let score_action_hash = match create_entry(&EntryTypes::Score(score_to_create)) {
        Ok(hash) => {
            debug!("[score.rs] create_score: create_entry for Score successful, action hash: {:?}", hash);
            hash
        }
        Err(e) => {
            debug!("[score.rs] create_score: create_entry for Score failed: {:?}", e);
            return Err(e);
        }
    };

    // Link the Score action hash from the Player's pubkey.
    create_link(
        input.player.clone(),
        score_action_hash.clone(),
        LinkTypes::PlayerToScores, 
        (),
    )?;

    // Link the Score action hash from the original game's action hash.
    create_link(
        input.game_id.clone(), 
        score_action_hash.clone(),
        LinkTypes::GameToScores, 
        (),
    )?;

    // Retrieve and return the created Score record.
    let record = get(score_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find the newly created Score".to_string())))?;
    debug!("[score.rs] create_score: Successfully created score, returning record for action: {:?}", record.action_hashed().hash);
    Ok(record)
}

// --- Other Score CRUD functions ---

#[hdk_extern]
pub fn get_scores_for_game(game_id: ActionHash) -> ExternResult<Vec<Record>> {
    // Ensure game_id is valid first? Optional.
     let _ = get_game_hash_by_id(&game_id)?
         .ok_or(wasm_error!(WasmErrorInner::Guest(format!("Game ID does not exist: {}", game_id))))?;


    let links = get_links(
        GetLinksInputBuilder::try_new(game_id, LinkTypes::GameToScores)? // Use the new link type
            .build(),
    )?;

    let get_inputs: Vec<GetInput> = links
        .into_iter()
        .filter_map(|link| link.target.into_action_hash())
        .map(|ah| GetInput::new(ah.into(), GetOptions::default()))
        .collect();

    if get_inputs.is_empty() {
        return Ok(vec![]);
    }

    let records = HDK.with(|hdk| hdk.borrow().get(get_inputs))?;
    Ok(records.into_iter().flatten().collect())
}


#[hdk_extern]
pub fn get_scores_for_player(player: AgentPubKey) -> ExternResult<Vec<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(player, LinkTypes::PlayerToScores)? // Correct link type
            .build()
    )?;

     let get_inputs: Vec<GetInput> = links
        .into_iter()
        .filter_map(|link| link.target.into_action_hash())
        .map(|ah| GetInput::new(ah.into(), GetOptions::default()))
        .collect();

    if get_inputs.is_empty() {
        return Ok(vec![]);
    }

    let records = HDK.with(|hdk| hdk.borrow().get(get_inputs))?;
    Ok(records.into_iter().flatten().collect())
}
