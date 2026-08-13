use hdk::prelude::*;
use crate::Signal;
use ping_2_pong_integrity::{ChatMessage, EntryTypes, LinkTypes};

#[hdk_extern]
pub fn send_global_chat_message(content: String) -> ExternResult<()> {
    let _ = crate::signals::grant_remote_signal_cap();
    let my_agent_info = agent_info()?;
    let my_pub_key = my_agent_info.agent_initial_pubkey.clone();
    let now_timestamp = sys_time()?;

    // 1. Persist ChatMessage entry on DHT chain
    let chat_entry = ChatMessage {
        sender: my_pub_key.clone(),
        content: content.clone(),
        timestamp: now_timestamp,
    };
    let action_hash = create_entry(&EntryTypes::ChatMessage(chat_entry))?;

    let anchor_path = Path::from("all_chat_messages");
    let anchor_hash = anchor_path.path_entry_hash()?;
    create_link(
        anchor_hash,
        action_hash,
        LinkTypes::AllChatMessagesAnchorToMessage,
        (),
    )?;

    // 2. Prepare signal for real-time delivery
    let signal = Signal::GlobalChatMessage {
        timestamp: now_timestamp,
        sender: my_pub_key.clone(),
        content,
    };

    // Emit locally for sender's UI
    emit_signal(&signal)?;

    let signal_io = ExternIO::encode(&signal).map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;

    // 3. Get all online & registered players for P2P delivery
    let mut recipients = crate::game::get_online_users(())?;
    if let Ok(all_players) = crate::player::get_all_player_pubkeys(()) {
        for agent_pubkey in all_players {
            if !recipients.contains(&agent_pubkey) {
                recipients.push(agent_pubkey);
            }
        }
    }

    // 4. Send to all other players via call_remote
    for target_agent_key in recipients {
        if target_agent_key != my_pub_key {
            let mut attempt = 0;
            while attempt < 3 {
                match call_remote(
                    target_agent_key.clone(),
                    "ping_2_pong",
                    "receive_remote_signal".into(),
                    None,
                    signal_io.clone()
                ) {
                    Ok(ZomeCallResponse::Ok(_io)) => {
                        info!("Successfully sent global chat message to {:?}", target_agent_key);
                        break;
                    }
                    Ok(ZomeCallResponse::Unauthorized(_cell_id, _zome, _func, _agent)) => {
                        warn!("Unauthorized remote call to {:?}: cap grant missing", target_agent_key);
                        break;
                    }
                    Ok(ZomeCallResponse::NetworkError(err)) => {
                        warn!("Attempt {} network error calling {:?}: {}", attempt + 1, target_agent_key, err);
                        attempt += 1;
                    }
                    Err(e) => {
                        warn!("Attempt {} failed to send global chat message to {:?}: {:?}", attempt + 1, target_agent_key, e);
                        attempt += 1;
                    }
                    _ => { break; }
                }
            }
        }
    }
    Ok(())
}

#[hdk_extern]
pub fn get_latest_chat_messages(_: ()) -> ExternResult<Vec<Signal>> {
    let anchor_path = Path::from("all_chat_messages");
    let anchor_hash = anchor_path.path_entry_hash()?;

    let links = get_links(
        LinkQuery::try_new(anchor_hash, LinkTypes::AllChatMessagesAnchorToMessage)?,
        GetStrategy::default(),
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
    let mut messages: Vec<Signal> = Vec::new();

    for record in records.into_iter().flatten() {
        if let Ok(Some(chat_entry)) = record.entry().to_app_option::<ChatMessage>() {
            messages.push(Signal::GlobalChatMessage {
                timestamp: chat_entry.timestamp,
                sender: chat_entry.sender,
                content: chat_entry.content,
            });
        }
    }

    // Sort messages by timestamp ascending
    messages.sort_by_key(|m| match m {
        Signal::GlobalChatMessage { timestamp, .. } => *timestamp,
        _ => Timestamp::from_micros(0),
    });

    // Retain up to the latest 100 messages
    if messages.len() > 100 {
        messages = messages.split_off(messages.len() - 100);
    }

    Ok(messages)
}
