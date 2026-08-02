use hdk::prelude::*;
use crate::Signal;

#[hdk_extern]
pub fn send_global_chat_message(content: String) -> ExternResult<()> {
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

    // 2. Get all online players
    let recipients = crate::game::get_online_users(())?;

    // 3. Send to all other players via call_remote
    for target_agent_key in recipients {
        if target_agent_key != my_pub_key { // Don't send to self again via call_remote
            match call_remote(
                target_agent_key.clone(),
                "ping_2_pong", // Zome name
                "receive_remote_signal".into(),
                None, // Unrestricted cap grant assumed for receive_remote_signal
                signal.clone() // Clone signal for each call
            ) {
                Ok(res) => {
                    info!("Successfully sent global chat message to {:?}: {:?}", target_agent_key, res);
                }
                Err(e) => {
                    warn!("Failed to send global chat message to {:?}: {:?}", target_agent_key, e);
                }
            }
        }
    }
    Ok(())
}
