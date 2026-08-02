// ─── dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/invitations.rs ───
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
        inviter: agent_info()?.agent_initial_pubkey,
        message: payload.message.clone(),
    };

    // 1) show it in *my* UI
    emit_signal(&signal)?;

    // 2) call remote invitee
    match call_remote(
        payload.invitee.clone(),     // who to call
        "ping_2_pong",               // zome
        "receive_remote_signal".into(),
        None,                        // unrestricted cap
        signal,                      // payload
    ) {
        Ok(res) => {
            info!("Successfully sent remote signal to {:?}: {:?}", payload.invitee, res);
        }
        Err(e) => {
            warn!("Failed to send remote invitation to {:?}: {:?}", payload.invitee, e);
            return Err(wasm_error!(format!("Failed to send remote invitation: {:?}", e)));
        }
    }

    Ok(())
}

/// Player-2 clicks **Accept** in the UI
#[hdk_extern]
pub fn accept_invitation(payload: AcceptInvitationPayload) -> ExternResult<()> {
    // delegate, then discard the Record so our signature stays `()`
    join_game(payload.game_id).map(|_updated_record| ())
    //                └──────── throw the value away, keep only the `Ok`
}
