// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/score.rs
use hdk::prelude::*;
use ping_2_pong_integrity::*;
use crate::utils::get_game_hash_by_id; // Use helper
use ping_2_pong_integrity::game::GameStatus;

// Maximum allowed score points.
const MAX_POINTS: u32 = 10000; // Keep high for flexibility, game logic enforces 10

#[hdk_extern]
pub fn create_score(score: Score) -> ExternResult<Record> {
    // --- Validation ---
    // Ensure the game_id corresponds to an actual Game entry
    // Note: game_id in Score struct should be the original ActionHash of the game creation
    let game_action_hash = get_game_hash_by_id(&score.game_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!("Game ID does not exist: {}", score.game_id))))?;

    // Fetch the *latest* game state record to check status
    let game_record = crate::game::get_latest_game(game_action_hash)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found".into())))?;
    let game = game_record
        .entry()
        .to_app_option::<Game>()
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid Game entry format".into())))?;

    // Ensure the game status is Finished before recording score
    if game.game_status != GameStatus::Finished {
        return Err(wasm_error!(WasmErrorInner::Guest("Scores can only be recorded for 'Finished' games".into())));
    }

    // Ensure the score is being assigned to a player who was actually in the game.
    if score.player != game.player_1 && game.player_2.as_ref() != Some(&score.player) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Score must be assigned to a player who participated in the game".into()
        )));
    }

    // Ensure caller is one of the players in the game? Or allow anyone to record score?
    // Let's allow anyone for now, assuming UI calls this after game ends for both players.
    // let my_pub_key = agent_info()?.agent_latest_pubkey;
    // if my_pub_key != game.player_1 && game.player_2.as_ref() != Some(&my_pub_key) {
    //     return Err(wasm_error!(WasmErrorInner::Guest("Only game participants can record the score".into())));
    // }


    // Validate that the score points are within a reasonable range.
    if score.player_points > MAX_POINTS { // MAX_POINTS is high, maybe check against game win condition?
        warn!("Score points {} exceed MAX_POINTS {}", score.player_points, MAX_POINTS);
        // Allow high scores for now, UI/game logic should enforce game rules like first to 10.
        // return Err(wasm_error!(WasmErrorInner::Guest("Player points exceed the maximum allowed".into())));
    }
     if score.player_points > 100 { // Add a more reasonable sanity check
         warn!("Recorded score {} seems high.", score.player_points);
     }
     // --- End Validation ---


    // Create the Score entry.
    let score_action_hash = create_entry(&EntryTypes::Score(score.clone()))?;

    // Link the Score action hash from the Player's pubkey.
    create_link(
        score.player.clone(),
        score_action_hash.clone(),
        LinkTypes::PlayerToScores, // Changed from ScoreToPlayer based on convention BaseToTargets
        (),
    )?;

    // Link the Score action hash from the original game's action hash.
    create_link(
        score.game_id.clone(), // Base is the original game action hash
        score_action_hash.clone(),
        LinkTypes::GameToScores, // Use a more descriptive name if possible, or reuse ScoreUpdates? Let's define GameToScores
        // LinkTypes::ScoreUpdates, // ScoreUpdates implies linking Score revisions, not linking *to* a score.
        (),
    )?;

    // Retrieve and return the created Score record.
    let record = get(score_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find the newly created Score".to_string())))?;
    Ok(record)
}

// --- Other Score CRUD functions ---
// get_latest_score/get_original_score/update_score etc. rely on ScoreUpdates links.
// The create_score now uses GameToScores. If updates to scores are needed,
// we might need ScoreUpdates links from the original Score action hash.
// Let's assume Scores are immutable for now, matching the design intent (record final score).
// We can remove update/delete score functions if scores are immutable.

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


// REMOVE functions related to Score updates and deletes if score is immutable post-creation
/*
#[hdk_extern]
pub fn get_latest_score(original_score_hash: ActionHash) -> ExternResult<Option<Record>> { ... }

#[hdk_extern]
pub fn get_original_score(original_score_hash: ActionHash) -> ExternResult<Option<Record>> { ... }

#[hdk_extern]
pub fn get_all_revisions_for_score(original_score_hash: ActionHash) -> ExternResult<Vec<Record>> { ... }

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateScoreInput { ... }

#[hdk_extern]
pub fn update_score(input: UpdateScoreInput) -> ExternResult<Record> { ... }

#[hdk_extern]
pub fn delete_score(original_score_hash: ActionHash) -> ExternResult<ActionHash> { ... }

#[hdk_extern]
pub fn get_all_deletes_for_score(original_score_hash: ActionHash) -> ExternResult<Option<Vec<SignedActionHashed>>> { ... }

#[hdk_extern]
pub fn get_oldest_delete_for_score(original_score_hash: ActionHash) -> ExternResult<Option<SignedActionHashed>> { ... }

#[hdk_extern]
pub fn get_deleted_scores_for_player(player: AgentPubKey) -> ExternResult<Vec<(SignedActionHashed, Vec<SignedActionHashed>)>> { ... }
*/

// get_game_hash_by_id was removed (now in utils)