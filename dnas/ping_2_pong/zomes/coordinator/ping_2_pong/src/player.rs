use hdk::prelude::*;
use ping_2_pong_integrity::*;
use crate::utils::anchor_for; 

// Helper function to check if a player name is unique using the PlayerNameToPlayer link
pub fn is_player_name_unique(player_name: &str) -> ExternResult<bool> {
    let name_anchor = anchor_for(&player_name.to_lowercase())?;
    let links = get_links(
        GetLinksInputBuilder::try_new(name_anchor, LinkTypes::PlayerNameToPlayer)?
            .build(),
    )?;
    Ok(links.is_empty())
}

#[hdk_extern]
pub fn create_player(player: Player) -> ExternResult<Record> {
    // --- Validations ---
     let my_pub_key = agent_info()?.agent_latest_pubkey;
     if player.player_key != my_pub_key {
         return Err(wasm_error!(WasmErrorInner::Guest("Player profile can only be created by the player themselves".into())));
     }
    if !is_player_name_unique(&player.player_name)? {
        return Err(wasm_error!(WasmErrorInner::Guest(format!( "Player name '{}' is already taken", player.player_name ))));
    }
     let existing_links = get_links(
        GetLinksInputBuilder::try_new(player.player_key.clone(), LinkTypes::PlayerToPlayers)?.build(),
     )?;
     if !existing_links.is_empty() {
         return Err(wasm_error!(WasmErrorInner::Guest("Player profile already exists for this agent".into())));
     }
    // --- End Validations ---

    let player_action_hash = create_entry(&EntryTypes::Player(player.clone()))?;
    create_link( player.player_key.clone(), player_action_hash.clone(), LinkTypes::PlayerToPlayers, (), )?;
    let name_anchor = anchor_for(&player.player_name.to_lowercase())?;
    create_link( name_anchor, player_action_hash.clone(), LinkTypes::PlayerNameToPlayer, (), )?;

    // Link player to the "all_players" anchor
    const ALL_PLAYERS_ANCHOR_STR: &str = "all_players";
    let all_players_path = Path::from(ALL_PLAYERS_ANCHOR_STR);
    let all_players_anchor_hash = all_players_path.path_entry_hash()?;
    create_link(
        all_players_anchor_hash.clone(), 
        player.player_key.clone(),       
        LinkTypes::AllPlayersAnchorToAgentPubKey,
        LinkTag::new(vec![]) 
    )?;

    let record = get(player_action_hash.clone(), GetOptions::default())?.ok_or(wasm_error!( WasmErrorInner::Guest("Could not find the newly created Player".to_string()) ))?;
    Ok(record)
}

// --- Other Player CRUD functions ---

#[hdk_extern]
pub fn get_latest_player(original_player_hash: ActionHash) -> ExternResult<Option<Record>> {
    let links = get_links( GetLinksInputBuilder::try_new(original_player_hash.clone(), LinkTypes::PlayerUpdates)?.build(), )?;
    let latest_link = links .into_iter() .max_by(|a, b| a.timestamp.cmp(&b.timestamp));
    let latest_player_hash = match latest_link {
        Some(link) => link.target.clone().into_action_hash() .ok_or(wasm_error!(WasmErrorInner::Guest( "No action hash associated with link".to_string() )))?,
        None => original_player_hash.clone(),
    };
    get(latest_player_hash, GetOptions::default())
}

#[hdk_extern]
pub fn get_original_player(original_player_hash: ActionHash) -> ExternResult<Option<Record>> {
    let Some(details) = get_details(original_player_hash, GetOptions::default())? else { return Ok(None); };
    match details {
        Details::Record(details) => Ok(Some(details.record)),
        _ => Err(wasm_error!(WasmErrorInner::Guest( "Malformed get details response".to_string() ))),
    }
}

#[hdk_extern]
pub fn get_all_revisions_for_player(original_player_hash: ActionHash) -> ExternResult<Vec<Record>> {
    let Some(original_record) = get_original_player(original_player_hash.clone())? else { return Ok(vec![]); };
    let links = get_links( GetLinksInputBuilder::try_new(original_player_hash.clone(), LinkTypes::PlayerUpdates)?.build(), )?;
    let get_input: Vec<GetInput> = links .into_iter() .map(|link| { Ok(GetInput::new( link.target .into_action_hash() .ok_or(wasm_error!(WasmErrorInner::Guest( "No action hash associated with link".to_string() )))? .into(), GetOptions::default(), )) }) .collect::<ExternResult<Vec<GetInput>>>()?;
    if get_input.is_empty() { return Ok(vec![original_record]); }
    let records_result = HDK.with(|hdk| hdk.borrow().get(get_input));
     let records = match records_result {
      Ok(records) => records,
      Err(e) => return Err(wasm_error!(WasmErrorInner::Guest(format!("Failed to get records: {:?}", e))))
    };
    let mut revision_records: Vec<Record> = records.into_iter().flatten().collect();
    revision_records.insert(0, original_record);
    Ok(revision_records)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdatePlayerInput {
    pub original_player_hash: ActionHash,
    pub previous_player_hash: ActionHash,
    pub updated_player: Player,
}

#[hdk_extern]
pub fn update_player(input: UpdatePlayerInput) -> ExternResult<Record> {
     let my_pub_key = agent_info()?.agent_latest_pubkey;
     let original_record = get(input.original_player_hash.clone(), GetOptions::default())? .ok_or(wasm_error!(WasmErrorInner::Guest("Original Player record not found".into())))?;

     let original_player = original_record.entry().to_app_option::<Player>()
         .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Deserialization error: {:?}", e))))? 
         .ok_or(wasm_error!(WasmErrorInner::Guest("Malformed original Player entry (None)".into())))?; 

     if original_player.player_key != my_pub_key {
         return Err(wasm_error!(WasmErrorInner::Guest("Cannot update another player's profile".into())));
     }
     if input.updated_player.player_name != original_player.player_name {
         if !is_player_name_unique(&input.updated_player.player_name)? {
             return Err(wasm_error!(WasmErrorInner::Guest(format!( "New player name '{}' is already taken", input.updated_player.player_name ))));
         }
         warn!("Player name change detected, but PlayerNameToPlayer link update is not implemented.");
     }

    let updated_player_hash = update_entry(input.previous_player_hash.clone(), &input.updated_player)?;
    create_link( input.original_player_hash.clone(), updated_player_hash.clone(), LinkTypes::PlayerUpdates, (), )?;
    let record = get(updated_player_hash.clone(), GetOptions::default())?.ok_or(wasm_error!( WasmErrorInner::Guest("Could not find the newly updated Player".to_string()) ))?;
    Ok(record)
}

#[hdk_extern]
pub fn delete_player(original_player_hash: ActionHash) -> ExternResult<ActionHash> {
    let my_pub_key = agent_info()?.agent_latest_pubkey;
    let details = get_details(original_player_hash.clone(), GetOptions::default())?.ok_or( wasm_error!(WasmErrorInner::Guest("Player not found".to_string())), )?;
    let record = match details { Details::Record(details) => details.record, _ => return Err(wasm_error!(WasmErrorInner::Guest( "Malformed get details response".to_string() ))), };

    let player = record.entry().to_app_option::<Player>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Deserialization error: {:?}", e))))? 
        .ok_or(wasm_error!(WasmErrorInner::Guest("Player record has no entry (None)".to_string())))?; 

    if player.player_key != my_pub_key {
        return Err(wasm_error!(WasmErrorInner::Guest("Cannot delete another player's profile".into())));
    }

    // Delete links
    let links_agent = get_links( GetLinksInputBuilder::try_new(player.player_key.clone(), LinkTypes::PlayerToPlayers)?.build(), )?;
    for link in links_agent { if let Some(action_hash) = link.target.into_action_hash() { if action_hash == original_player_hash { delete_link(link.create_link_hash)?; } } }
    let name_anchor = anchor_for(&player.player_name.to_lowercase())?;
    let links_name = get_links( GetLinksInputBuilder::try_new(name_anchor, LinkTypes::PlayerNameToPlayer)?.build(), )?;
     for link in links_name { if let Some(action_hash) = link.target.into_action_hash() { if action_hash == original_player_hash { delete_link(link.create_link_hash)?; } } }

    // Delete entry
    delete_entry(original_player_hash)
}

#[hdk_extern]
pub fn get_all_deletes_for_player( original_player_hash: ActionHash, ) -> ExternResult<Option<Vec<SignedActionHashed>>> {
    let Some(details) = get_details(original_player_hash, GetOptions::default())? else { return Ok(None); };
    match details { Details::Entry(_) => Err(wasm_error!(WasmErrorInner::Guest("Malformed details".into()))), Details::Record(record_details) => Ok(Some(record_details.deletes)), }
}

#[hdk_extern]
pub fn get_oldest_delete_for_player( original_player_hash: ActionHash, ) -> ExternResult<Option<SignedActionHashed>> {
    let Some(mut deletes) = get_all_deletes_for_player(original_player_hash)? else { return Ok(None); };
    deletes.sort_by(|a, b| a.action().timestamp().cmp(&b.action().timestamp()));
    Ok(deletes.first().cloned())
}

#[hdk_extern]
pub fn get_player_profile_hash_for_agent(player_agent_key: AgentPubKey) -> ExternResult<Vec<Link>> {
    get_links(GetLinksInputBuilder::try_new(player_agent_key, LinkTypes::PlayerToPlayers)?.build())
}

#[hdk_extern]
pub fn get_deleted_player_links_for_agent( player_agent_key: AgentPubKey, ) -> ExternResult<Vec<(SignedActionHashed, Vec<SignedActionHashed>)>> {
    let details = get_link_details( player_agent_key, LinkTypes::PlayerToPlayers, None, GetOptions::default(), )?;
    Ok(details .into_inner() .into_iter() .filter(|(_, deletes)| !deletes.is_empty()) .collect())
}

#[hdk_extern]
pub fn get_player_by_name(player_name: String) -> ExternResult<Option<Record>> {
    let name_anchor = anchor_for(&player_name.to_lowercase())?;
    let links = get_links( GetLinksInputBuilder::try_new(name_anchor, LinkTypes::PlayerNameToPlayer)?.build(), )?;
    if let Some(link) = links.into_iter().next() {
        if let Some(action_hash) = link.target.into_action_hash() {
            get_original_player(action_hash)
        } else { Ok(None) }
    } else { Ok(None) }
}

#[hdk_extern]
pub fn get_all_player_pubkeys(_: ()) -> ExternResult<Vec<AgentPubKey>> {
    const ALL_PLAYERS_ANCHOR_STR: &str = "all_players";
    let all_players_path = Path::from(ALL_PLAYERS_ANCHOR_STR);
    let all_players_anchor_hash = all_players_path.path_entry_hash()?;

    let links = get_links(
        GetLinksInputBuilder::try_new(all_players_anchor_hash.clone(), LinkTypes::AllPlayersAnchorToAgentPubKey)?
        .build()
    )?;
    
    let pub_keys: Vec<AgentPubKey> = links
        .into_iter()
        .filter_map(|link| link.target.into_agent_pub_key()) 
        .collect();
        
    Ok(pub_keys)
}

#[hdk_extern]
pub fn get_player_profile_by_agent_key(agent_key: AgentPubKey) -> ExternResult<Option<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(agent_key.clone(), LinkTypes::PlayerToPlayers)?.build()
    )?;

    if let Some(link) = links.into_iter().next() { 
        match link.target.into_action_hash() {
            Some(action_hash) => {
                get(action_hash, GetOptions::default()) 
            }
            None => {
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}