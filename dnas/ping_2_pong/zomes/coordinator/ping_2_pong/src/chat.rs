use hdk::prelude::*;
use crate::Signal;

#[hdk_extern]
pub fn send_global_chat_message(content: String) -> ExternResult<()> {
    let _ = crate::signals::grant_remote_signal_cap();
    let my_agent_info = agent_info()?;
    let my_pub_key = my_agent_info.agent_initial_pubkey.clone();
    let now_timestamp = sys_time()?;

    let signal = Signal::GlobalChatMessage {
        timestamp: now_timestamp,
        sender: my_pub_key.clone(),
        content,
    };

    // 1. Emit locally for sender's UI
    emit_signal(&signal)?;

    let signal_io = ExternIO::encode(&signal).map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;

    // 2. Get all online players & all registered players
    let mut recipients = crate::game::get_online_users(())?;
    if let Ok(all_players) = crate::player::get_all_player_pubkeys(()) {
        for agent_pubkey in all_players {
            if !recipients.contains(&agent_pubkey) {
                recipients.push(agent_pubkey);
            }
        }
    }

    // 3. Send to all other players via call_remote with retries
    for target_agent_key in recipients {
        if target_agent_key != my_pub_key { // Don't send to self again via call_remote
            let mut attempt = 0;
            while attempt < 3 {
                match call_remote(
                    target_agent_key.clone(),
                    "ping_2_pong", // Zome name
                    "receive_remote_signal".into(),
                    None, // Unrestricted cap grant assumed for receive_remote_signal
                    signal_io.clone() // Clone signal_io for each call
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
