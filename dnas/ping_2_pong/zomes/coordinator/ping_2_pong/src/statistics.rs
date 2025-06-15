use hdk::prelude::*;
use ping_2_pong_integrity::*; 
use crate::utils::get_game_hash_by_id; 
use ping_2_pong_integrity::game::GameStatus;
use crate::player::get_all_player_pubkeys; 
use crate::score::get_scores_for_player;   

#[derive(Serialize, Deserialize, Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct LeaderboardEntry {
    pub player_key: AgentPubKey,
    pub total_points: u32,
    pub games_played: u32,
}

// Define maximum allowed values as constants - Keep these reasonable sanity checks
const MAX_LATENCY: u32 = 30000; // 30 seconds - high, but allows for network issues
const MAX_SCORE_VALIDATION_TIME: u32 = 60000; // 60 seconds 
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

     // Ensure caller is one of the players
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
        statistics.game_id.clone(), 
        statistics_action_hash.clone(),
        LinkTypes::GameToStatistics, 
        (),
    )?;


    // Retrieve and return the created Statistics record.
    let record = get(statistics_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not find the newly created Statistics".to_string()
        )))?;

    Ok(record)
}

// Add function to get statistics for a game.

#[hdk_extern]
pub fn get_statistics_for_game(game_id: ActionHash) -> ExternResult<Vec<Record>> {
    // Ensure game_id is valid first? Optional.
     let _ = get_game_hash_by_id(&game_id)?
         .ok_or(wasm_error!(WasmErrorInner::Guest(format!("Game ID does not exist: {}", game_id))))?;

    let links = get_links(
        GetLinksInputBuilder::try_new(game_id, LinkTypes::GameToStatistics)? 
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
pub fn get_leaderboard_data(_: ()) -> ExternResult<Vec<LeaderboardEntry>> {
    // 1. Get all player public keys
    let all_player_keys = get_all_player_pubkeys(())?;

    let mut leaderboard_entries: Vec<LeaderboardEntry> = Vec::new();

    // 2. For each player, get their scores and aggregate
    for player_key in all_player_keys {
        let score_records = get_scores_for_player(player_key.clone())?;

        let mut total_points: u32 = 0;
        let mut games_played: u32 = 0;

        for record in score_records {
            match record.entry().to_app_option::<ping_2_pong_integrity::Score>() { 
                Ok(Some(score_entry)) => {
                    total_points += score_entry.player_points;
                    games_played += 1;
                }
                Ok(None) => {
                    warn!("Score record for player {:?} has no app entry after deserialization attempt.", player_key);
                }
                Err(e) => {
                    warn!("Failed to deserialize Score app entry for player {:?}: {:?}", player_key, e);
                }
            }
        }
        
        // Add player to leaderboard even if they have 0 games/points
        leaderboard_entries.push(LeaderboardEntry {
            player_key: player_key.clone(),
            total_points,
            games_played,
        });
    }

    // 3. Sort the leaderboard
    leaderboard_entries.sort_by(|a, b| {
        b.total_points.cmp(&a.total_points) // Sort by total_points descending
            .then_with(|| a.games_played.cmp(&b.games_played)) // Then by games_played ascending
            .then_with(|| a.player_key.cmp(&b.player_key)) // Then by player_key for consistent tie-breaking
    });

    Ok(leaderboard_entries)
}