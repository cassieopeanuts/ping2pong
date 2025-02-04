use hdk::prelude::*;
use ping_2_pong_integrity::*;
use crate::game::get_game_hash_by_id;

// Define maximum allowed values as constants
const MAX_LATENCY: u32 = 100000; // in milliseconds
const MAX_SCORE_VALIDATION_TIME: u32 = 100000; // in milliseconds
const MAX_DHT_RESPONSE_TIME: u32 = 100000; // in milliseconds
const MAX_NETWORK_DELAY: u32 = 100000; // in milliseconds

#[hdk_extern]
pub fn create_statistics(statistics: Statistics) -> ExternResult<Record> {
    // Create the Statistics entry.
    let statistics_hash = create_entry(&EntryTypes::Statistics(statistics.clone()))?;
    let record = get(statistics_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not find the newly created Statistics".to_string()
        )))?;

    // Ensure the game_id exists by retrieving the corresponding game hash.
    let game_hash = match get_game_hash_by_id(&statistics.game_id)? {
        Some(hash) => hash,
        None => return Err(wasm_error!(WasmErrorInner::Guest("Game ID does not exist".into()))),
    };

    // Fetch the Game record.
    let game_record = get(game_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found".into())))?;
    let game = game_record
        .entry()
        .to_app_option::<Game>()
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid game entry".into())))?;

    // Ensure the game is Finished.
    if game.game_status != GameStatus::Finished {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Statistics can only be recorded for finished games".into()
        )));
    }

    // Validate statistical metrics ranges.
    if statistics.signal_latency > MAX_LATENCY {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Signal latency exceeds maximum allowed value".into()
        )));
    }
    if statistics.score_validation_time > MAX_SCORE_VALIDATION_TIME {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Score validation time exceeds maximum allowed value".into()
        )));
    }
    if statistics.dht_response_time > MAX_DHT_RESPONSE_TIME {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "DHT response time exceeds maximum allowed value".into()
        )));
    }
    if statistics.network_delay > MAX_NETWORK_DELAY {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Network delay exceeds maximum allowed value".into()
        )));
    }

    // Return the created Statistics record.
    Ok(record)
}

#[hdk_extern]
pub fn get_latest_statistics(original_statistics_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(
            original_statistics_hash.clone(),
            LinkTypes::StatisticsUpdates,
        )?
        .build(),
    )?;
    let latest_link = links
        .into_iter()
        .max_by(|link_a, link_b| link_a.timestamp.cmp(&link_b.timestamp));
    let latest_statistics_hash = match latest_link {
        Some(link) => {
            link.target
                .clone()
                .into_action_hash()
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "No action hash associated with link".to_string()
                )))?
        }
        None => original_statistics_hash.clone(),
    };
    get(latest_statistics_hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_original_statistics(
    original_statistics_hash: ActionHash,
) -> ExternResult<Option<Record>> {
    let Some(details) = get_details(original_statistics_hash, GetOptions::default())? else {
        return Ok(None);
    };
    match details {
        Details::Record(details) => Ok(Some(details.record)),
        _ => Err(wasm_error!(WasmErrorInner::Guest(
            "Malformed get details response".to_string()
        ))),
    }
}

#[hdk_extern]
pub fn get_all_revisions_for_statistics(
    original_statistics_hash: ActionHash,
) -> ExternResult<Vec<Record>> {
    let Some(original_record) = get_original_statistics(original_statistics_hash.clone())? else {
        return Ok(vec![]);
    };
    let links = get_links(
        GetLinksInputBuilder::try_new(
            original_statistics_hash.clone(),
            LinkTypes::StatisticsUpdates,
        )?
        .build(),
    )?;
    let get_input: Vec<GetInput> = links
        .into_iter()
        .map(|link| {
            Ok(GetInput::new(
                link.target
                    .into_action_hash()
                    .ok_or(wasm_error!(WasmErrorInner::Guest(
                        "No action hash associated with link".to_string()
                    )))?
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
pub struct UpdateStatisticsInput {
    pub original_statistics_hash: ActionHash,
    pub previous_statistics_hash: ActionHash,
    pub updated_statistics: Statistics,
}

#[hdk_extern]
pub fn update_statistics(input: UpdateStatisticsInput) -> ExternResult<Record> {
    let updated_statistics_hash = update_entry(input.previous_statistics_hash.clone(), &input.updated_statistics)?;
    create_link(
        input.original_statistics_hash.clone(),
        updated_statistics_hash.clone(),
        LinkTypes::StatisticsUpdates,
        (),
    )?;
    let record = get(updated_statistics_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not find the newly updated Statistics".to_string()
        )))?;
    Ok(record)
}

#[hdk_extern]
pub fn delete_statistics(original_statistics_hash: ActionHash) -> ExternResult<ActionHash> {
    delete_entry(original_statistics_hash)
}

#[hdk_extern]
pub fn get_all_deletes_for_statistics(
    original_statistics_hash: ActionHash,
) -> ExternResult<Option<Vec<SignedActionHashed>>> {
    let Some(details) = get_details(original_statistics_hash, GetOptions::default())? else {
        return Ok(None);
    };
    match details {
        Details::Entry(_) => Err(wasm_error!(WasmErrorInner::Guest("Malformed details".into()))),
        Details::Record(record_details) => Ok(Some(record_details.deletes)),
    }
}

#[hdk_extern]
pub fn get_oldest_delete_for_statistics(
    original_statistics_hash: ActionHash,
) -> ExternResult<Option<SignedActionHashed>> {
    let Some(mut deletes) = get_all_deletes_for_statistics(original_statistics_hash)? else {
        return Ok(None);
    };
    deletes.sort_by(|delete_a, delete_b| {
        delete_a.action().timestamp().cmp(&delete_b.action().timestamp())
    });
    Ok(deletes.first().cloned())
}
