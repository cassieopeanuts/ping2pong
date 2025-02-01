use hdk::prelude::*;
use ping_2_pong_integrity::*;

#[hdk_extern]
pub fn create_player(player: Player) -> ExternResult<Record> {
    // Check if player_name is unique
    if !is_player_name_unique(&player.player_name)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Player name must be unique".into(),
        )));
    }

    let player_hash = create_entry(&EntryTypes::Player(player.clone()))?;
    create_link(
        player.player_key.clone(),
        player_hash.clone(),
        LinkTypes::PlayerToPlayers,
        (),
    )?;

    // Create a link from player_name to player entry for quick lookup
    create_link(
        ping_2_pong_integrity::anchor_for(&player.player_name.to_lowercase())?,
        player_hash.clone(),
        LinkTypes::PlayerNameToPlayer,
        (),
    )?;
    
    let record = get(player_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly created Player".to_string())
    ))?;
    Ok(record)
}

#[hdk_extern]
pub fn get_latest_player(original_player_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(original_player_hash.clone(), LinkTypes::PlayerUpdates)?
            .build(),
    )?;
    let latest_link = links
        .into_iter()
        .max_by(|link_a, link_b| link_a.timestamp.cmp(&link_b.timestamp));
    let latest_player_hash = match latest_link {
        Some(link) => {
            link.target
                .clone()
                .into_action_hash()
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "No action hash associated with link".to_string()
                )))?
        }
        None => original_player_hash.clone(),
    };
    get(latest_player_hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_original_player(original_player_hash: ActionHash) -> ExternResult<Option<Record>> {
    let Some(details) = get_details(original_player_hash, GetOptions::default())? else {
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
pub fn get_all_revisions_for_player(original_player_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let Some(original_record) = get_original_player(original_player_hash.clone())? else {
        return Ok(vec![]);
    };
    let links = get_links(
        GetLinksInputBuilder::try_new(original_player_hash.clone(), LinkTypes::PlayerUpdates)?
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
pub struct UpdatePlayerInput {
    pub original_player_hash: ActionHash,
    pub previous_player_hash: ActionHash,
    pub updated_player: Player,
}

#[hdk_extern]
pub fn update_player(input: UpdatePlayerInput) -> ExternResult<Record> {
    let updated_player_hash =
        update_entry(input.previous_player_hash.clone(), &input.updated_player)?;
    create_link(
        input.original_player_hash.clone(),
        updated_player_hash.clone(),
        LinkTypes::PlayerUpdates,
        (),
    )?;
    let record = get(updated_player_hash.clone(), GetOptions::default())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly updated Player".to_string())
    ))?;
    Ok(record)
}

#[hdk_extern]
pub fn delete_player(original_player_hash: ActionHash) -> ExternResult<ActionHash> {
    let details = get_details(original_player_hash.clone(), GetOptions::default())?.ok_or(
        wasm_error!(WasmErrorInner::Guest("Player not found".to_string())),
    )?;
    let record = match details {
        Details::Record(details) => Ok(details.record),
        _ => Err(wasm_error!(WasmErrorInner::Guest(
            "Malformed get details response".to_string()
        ))),
    }?;
    let entry = record
        .entry()
        .as_option()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Player record has no entry".to_string()
        )))?;
    let player = <Player>::try_from(entry)?;
    let links = get_links(
        GetLinksInputBuilder::try_new(player.player_key.clone(), LinkTypes::PlayerToPlayers)?
            .build(),
    )?;
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if action_hash == original_player_hash {
                delete_link(link.create_link_hash)?;
            }
        }
    }
    delete_entry(original_player_hash)
}

#[hdk_extern]
pub fn get_all_deletes_for_player(
    original_player_hash: ActionHash,
) -> ExternResult<Option<Vec<SignedActionHashed>>> {
    let Some(details) = get_details(original_player_hash, GetOptions::default())? else {
        return Ok(None);
    };
    match details {
        Details::Entry(_) => Err(wasm_error!(WasmErrorInner::Guest(
            "Malformed details".into()
        ))),
        Details::Record(record_details) => Ok(Some(record_details.deletes)),
    }
}

#[hdk_extern]
pub fn get_oldest_delete_for_player(
    original_player_hash: ActionHash,
) -> ExternResult<Option<SignedActionHashed>> {
    let Some(mut deletes) = get_all_deletes_for_player(original_player_hash)? else {
        return Ok(None);
    };
    deletes.sort_by(|delete_a, delete_b| {
        delete_a
            .action()
            .timestamp()
            .cmp(&delete_b.action().timestamp())
    });
    Ok(deletes.first().cloned())
}

#[hdk_extern]
pub fn get_players_for_player(player: AgentPubKey) -> ExternResult<Vec<Link>> {
    get_links(GetLinksInputBuilder::try_new(player, LinkTypes::PlayerToPlayers)?.build())
}

#[hdk_extern]
pub fn get_deleted_players_for_player(
    player: AgentPubKey,
) -> ExternResult<Vec<(SignedActionHashed, Vec<SignedActionHashed>)>> {
    let details = get_link_details(
        player,
        LinkTypes::PlayerToPlayers,
        None,
        GetOptions::default(),
    )?;
    Ok(details
        .into_inner()
        .into_iter()
        .filter(|(_link, deletes)| !deletes.is_empty())
        .collect())
}
