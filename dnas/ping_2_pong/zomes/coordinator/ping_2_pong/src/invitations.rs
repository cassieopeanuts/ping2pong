use hdk::prelude::*;
use crate::{Signal, game::join_game,};

/// Data the UI passes in when one player invites another.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InvitationPayload {
    pub game_id: ActionHash,
    pub invitee: AgentPubKey,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AcceptInvitationPayload {
    pub game_id: ActionHash,
}

/// Player-to-player invitation (P1 ➜ P2)
#[hdk_extern]
pub fn send_invitation(payload: InvitationPayload) -> ExternResult<()> {
    // build the signal once
    let signal = Signal::GameInvitation {
        game_id: payload.game_id.clone(),
        inviter: agent_info()?.agent_latest_pubkey,
        message: payload.message.clone(),
    };

    // 1) show it in UI
    emit_signal(&signal)?;

    // 2) fire-and-forget to the invitee
    let _ = call_remote(
        payload.invitee,             
        "ping_2_pong",               
        "receive_remote_signal".into(),
        None,                        
        signal,                      
    );

    Ok(())
}

/// Player-2 clicks **Accept** in the UI
#[hdk_extern]
pub fn accept_invitation(payload: AcceptInvitationPayload) -> ExternResult<()> {
    join_game(payload.game_id).map(|_updated_record| ())
}
