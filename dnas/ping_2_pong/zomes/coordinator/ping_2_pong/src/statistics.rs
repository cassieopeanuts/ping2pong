// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/statistics.rs
use hdk::prelude::*;
use ping_2_pong_integrity::*;
use crate::utils::get_game_hash_by_id; // Use helper
use ping_2_pong_integrity::game::GameStatus;

// Define maximum allowed values as constants - Keep these reasonable sanity checks
const MAX_LATENCY: u32 = 30000; // 30 seconds - high, but allows for network issues
const MAX_SCORE_VALIDATION_TIME: u32 = 60000; // 60 seconds - likely irrelevant now
const MAX_DHT_RESPONSE_TIME: u32 = 60000; // 60 seconds
const MAX_NETWORK_DELAY: u32 = 30000; // 30 seconds

#[hdk_extern]
pub fn create_statistics(mut statistics: Statistics) -> ExternResult<Record> { // make mutable for timestamp

    // --- Validation ---
    // Ensure the game_id corresponds to an actual Game entry
    let game_action_hash = get_game_hash_by_id(&statistics.game_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!("Game ID does not exist: {}", statistics.game_id))))?;

    // Fetch the *latest* Game record to check status
    let game_record = crate::game::get_latest_game(game_action_hash)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found for statistics check".into())))?;
    let game = game_record
        .entry()
        .to_app_option::<Game>()
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid Game entry format for statistics check".into())))?;

    // Ensure the game is Finished.
    if game.game_status != GameStatus::Finished {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Statistics can only be recorded for 'Finished' games".into()
        )));
    }

     // Ensure caller is one of the players? Or allow anyone?
     // Let's restrict to players.
     let my_pub_key = agent_info()?.agent_latest_pubkey;
     if my_pub_key != game.player_1 && game.player_2.as_ref() != Some(&my_pub_key) {
         return Err(wasm_error!(WasmErrorInner::Guest("Only game participants can record statistics".into())));
     }

    // Validate statistical metrics ranges as basic sanity checks.
    // The actual measurement happens client-side.
    if statistics.signal_latency > MAX_LATENCY {
         warn!("Reported signal latency {} exceeds maximum sanity check {}", statistics.signal_latency, MAX_LATENCY);
        // return Err(wasm_error!(WasmErrorInner::Guest("Signal latency exceeds maximum allowed value".into())));
    }
    // 'score_validation_time' might be irrelevant if score saving is simple create_entry. Rename or remove?
    if statistics.score_validation_time > MAX_SCORE_VALIDATION_TIME {
        warn!("Reported score_validation_time {} exceeds maximum sanity check {}", statistics.score_validation_time, MAX_SCORE_VALIDATION_TIME);
       // return Err(wasm_error!(WasmErrorInner::Guest("Score validation time exceeds maximum allowed value".into())));
    }
    if statistics.dht_response_time > MAX_DHT_RESPONSE_TIME {
         warn!("Reported dht_response_time {} exceeds maximum sanity check {}", statistics.dht_response_time, MAX_DHT_RESPONSE_TIME);
       // return Err(wasm_error!(WasmErrorInner::Guest("DHT response time exceeds maximum allowed value".into())));
    }
    if statistics.network_delay > MAX_NETWORK_DELAY {
         warn!("Reported network_delay {} exceeds maximum sanity check {}", statistics.network_delay, MAX_NETWORK_DELAY);
       // return Err(wasm_error!(WasmErrorInner::Guest("Network delay exceeds maximum allowed value".into())));
    }

    // Set server-side timestamp
    statistics.timestamp = sys_time()?;
    // --- End Validation ---


    // Create the Statistics entry.
    let statistics_action_hash = create_entry(&EntryTypes::Statistics(statistics.clone()))?;

    // Link from the game_id ActionHash to the Statistics ActionHash
    create_link(
        statistics.game_id.clone(), // Base is original game action hash
        statistics_action_hash.clone(),
        LinkTypes::GameToStatistics, // Define a new link type
        (),
    )?;


    // Retrieve and return the created Statistics record.
    let record = get(statistics_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not find the newly created Statistics".to_string()
        )))?;

    Ok(record)
}


// Statistics are likely immutable once created. Remove update/delete functions.
// Add function to get statistics for a game.

#[hdk_extern]
pub fn get_statistics_for_game(game_id: ActionHash) -> ExternResult<Vec<Record>> {
    // Ensure game_id is valid first? Optional.
     let _ = get_game_hash_by_id(&game_id)?
         .ok_or(wasm_error!(WasmErrorInner::Guest(format!("Game ID does not exist: {}", game_id))))?;

    let links = get_links(
        GetLinksInputBuilder::try_new(game_id, LinkTypes::GameToStatistics)? // Use the new link type
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


// REMOVE functions related to Statistics updates and deletes
/*
#[hdk_extern]
pub fn get_latest_statistics(original_statistics_hash: ActionHash) -> ExternResult<Option<Record>> { ... }

#[hdk_extern]
pub fn get_original_statistics(original_statistics_hash: ActionHash) -> ExternResult<Option<Record>> { ... }

#[hdk_extern]
pub fn get_all_revisions_for_statistics(original_statistics_hash: ActionHash) -> ExternResult<Vec<Record>> { ... }

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateStatisticsInput { ... }

#[hdk_extern]
pub fn update_statistics(input: UpdateStatisticsInput) -> ExternResult<Record> { ... }

#[hdk_extern]
pub fn delete_statistics(original_statistics_hash: ActionHash) -> ExternResult<ActionHash> { ... } // Maybe keep delete?

#[hdk_extern]
pub fn get_all_deletes_for_statistics(original_statistics_hash: ActionHash) -> ExternResult<Option<Vec<SignedActionHashed>>> { ... }

#[hdk_extern]
pub fn get_oldest_delete_for_statistics(original_statistics_hash: ActionHash) -> ExternResult<Option<SignedActionHashed>> { ... }
*/