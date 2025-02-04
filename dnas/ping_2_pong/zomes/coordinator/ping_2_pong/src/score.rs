use hdk::prelude::*;
use ping_2_pong_integrity::*;

// Maximum allowed score points.
const MAX_POINTS: u32 = 10000;

#[hdk_extern]
pub fn create_score(score: Score) -> ExternResult<Record> {
    // Ensure the game_id exists and retrieve its ActionHash.
    let game_hash = match get_game_hash_by_id(&score.game_id)? {
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
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid Game entry".into())))?;

    // Ensure the game is active (InProgress or Finished).
    match game.game_status {
        GameStatus::InProgress | GameStatus::Finished => (),
        _ => return Err(wasm_error!(WasmErrorInner::Guest("Game is not active".into()))),
    }

    // Ensure the score is being assigned to a player in the game.
    if score.player != game.player_1 && score.player != game.player_2 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Score must be assigned to a player in the game".into()
        )));
    }

    // Validate that the score points are within the acceptable range.
    if score.player_points > MAX_POINTS {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Player points exceed the maximum allowed".into()
        )));
    }

    // Create the Score entry.
    let score_hash = create_entry(&EntryTypes::Score(score.clone()))?;

    // Link the Score to the Player.
    create_link(
        score.player.clone(),
        score_hash.clone(),
        LinkTypes::ScoreToPlayer,
        (),
    )?;

    // Link the Score to the original game.
    create_link(
        score.game_id.clone(),
        score_hash.clone(),
        LinkTypes::ScoreUpdates,
        (),
    )?;

    // Retrieve and return the created Score record.
    let record = get(score_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find the newly created Score".to_string())))?;
    Ok(record)
}

#[hdk_extern]
pub fn get_latest_score(original_score_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(original_score_hash.clone(), LinkTypes::ScoreUpdates)?
            .build(),
    )?;
    let latest_link = links
        .into_iter()
        .max_by(|link_a, link_b| link_a.timestamp.cmp(&link_b.timestamp));
    let latest_score_hash = match latest_link {
        Some(link) => {
            link.target
                .clone()
                .into_action_hash()
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "No action hash associated with link".to_string()
                )))?
        }
        None => original_score_hash.clone(),
    };
    get(latest_score_hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_original_score(original_score_hash: ActionHash) -> ExternResult<Option<Record>> {
    let Some(details) = get_details(original_score_hash, GetOptions::default())? else {
        return Ok(None);
    };
    match details {
        Details::Record(details) => Ok(Some(details.record)),
        _ => Err(wasm_error!(WasmErrorInner::Guest("Malformed get details response".to_string()))),
    }
}

#[hdk_extern]
pub fn get_all_revisions_for_score(original_score_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let Some(original_record) = get_original_score(original_score_hash.clone())? else {
        return Ok(vec![]);
    };
    let links = get_links(
        GetLinksInputBuilder::try_new(original_score_hash.clone(), LinkTypes::ScoreUpdates)?
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
pub struct UpdateScoreInput {
    pub original_score_hash: ActionHash,
    pub previous_score_hash: ActionHash,
    pub updated_score: Score,
}

#[hdk_extern]
pub fn update_score(input: UpdateScoreInput) -> ExternResult<Record> {
    let updated_score_hash = update_entry(input.previous_score_hash.clone(), &input.updated_score)?;
    create_link(
        input.original_score_hash.clone(),
        updated_score_hash.clone(),
        LinkTypes::ScoreUpdates,
        (),
    )?;
    let record = get(updated_score_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find the newly updated Score".to_string())))?;
    Ok(record)
}

#[hdk_extern]
pub fn delete_score(original_score_hash: ActionHash) -> ExternResult<ActionHash> {
    let details = get_details(original_score_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Score not found".to_string())))?;
    let record = match details {
        Details::Record(details) => Ok(details.record),
        _ => Err(wasm_error!(WasmErrorInner::Guest("Malformed get details response".to_string()))),
    }?;
    let entry = record
        .entry()
        .as_option()
        .ok_or(wasm_error!(WasmErrorInner::Guest("Score record has no entry".to_string())))?;
    let score = <Score>::try_from(entry)?;
    let links = get_links(
        GetLinksInputBuilder::try_new(score.player.clone(), LinkTypes::PlayerToScores)?.build(),
    )?;
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if action_hash == original_score_hash {
                delete_link(link.create_link_hash)?;
            }
        }
    }
    delete_entry(original_score_hash)
}

#[hdk_extern]
pub fn get_all_deletes_for_score(original_score_hash: ActionHash) -> ExternResult<Option<Vec<SignedActionHashed>>> {
    let Some(details) = get_details(original_score_hash, GetOptions::default())? else {
        return Ok(None);
    };
    match details {
        Details::Entry(_) => Err(wasm_error!(WasmErrorInner::Guest("Malformed details".into()))),
        Details::Record(record_details) => Ok(Some(record_details.deletes)),
    }
}

#[hdk_extern]
pub fn get_oldest_delete_for_score(original_score_hash: ActionHash) -> ExternResult<Option<SignedActionHashed>> {
    let Some(mut deletes) = get_all_deletes_for_score(original_score_hash)? else {
        return Ok(None);
    };
    deletes.sort_by(|delete_a, delete_b| delete_a.action().timestamp().cmp(&delete_b.action().timestamp()));
    Ok(deletes.first().cloned())
}

#[hdk_extern]
pub fn get_scores_for_player(player: AgentPubKey) -> ExternResult<Vec<Link>> {
    get_links(GetLinksInputBuilder::try_new(player, LinkTypes::PlayerToScores)?.build())
}

#[hdk_extern]
pub fn get_deleted_scores_for_player(player: AgentPubKey) -> ExternResult<Vec<(SignedActionHashed, Vec<SignedActionHashed>)>> {
    let details = get_link_details(
        player,
        LinkTypes::PlayerToScores,
        None,
        GetOptions::default(),
    )?;
    Ok(details.into_inner().into_iter().filter(|(_link, deletes)| !deletes.is_empty()).collect())
}

// Helper function to get game hash by game_id.
// (Note: Adjust this helper as needed for your anchor strategy.)
fn get_game_hash_by_id(game_id: &ActionHash) -> ExternResult<Option<ActionHash>> {
    // For example, use an anchor for "game_updates".
    let anchor = crate::utils::anchor_for("game_updates")?;
    let game_links = get_links(
        GetLinksInputBuilder::try_new(anchor, LinkTypes::GameUpdates)?.build(),
    )?;
    for link in game_links {
        let game_hash = link.target.into_action_hash().ok_or(
            wasm_error!(WasmErrorInner::Guest("Invalid game hash".to_string())),
        )?;
        let game_record = get(game_hash.clone(), GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found".to_string())))?;
        if let Some(game) = game_record
            .entry()
            .to_app_option::<Game>()
            .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        {
            if game.game_id == *game_id {
                return Ok(Some(game_hash.clone()));
            }
        }
    }
    Ok(None)
}
