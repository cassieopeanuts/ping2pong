use hdk::prelude::*;
use crate::{Signal, Game};

/// ───────────────────────── init helper ─────────────────────────
pub fn grant_remote_signal_cap() -> ExternResult<()> {
    create_cap_grant(CapGrantEntry {
        tag: "remote-signal".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(
            vec![("ping_2_pong".into(), "receive_remote_signal".into())]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
    })?;
    Ok(())
}

/// ──────────────────────── local re-emit ───────────────────────
#[hdk_extern]
pub fn receive_remote_signal(signal: Signal) -> ExternResult<()> {
    emit_signal(&signal)
}

/// ─────────────────────── payload structs ──────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaddleUpdatePayload {
    pub game_id:  ActionHash,
    pub paddle_y: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BallUpdatePayload {
    pub game_id: ActionHash,
    pub ball_x:  u32,
    pub ball_y:  u32,
    pub ball_dx: i32,
    pub ball_dy: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameOverPayload {
    pub game_id: ActionHash,
    pub winner:  Option<AgentPubKey>,
    pub score1:  u32,
    pub score2:  u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameAbandonedPayload {
    pub game_id: ActionHash,
}

/// Return the `Record` at the *tip* of an update chain
fn latest_record(original: &ActionHash) -> ExternResult<Record> {
    let mut current = original.clone();

    loop {
        let details = get_details(current.clone(), GetOptions::default())?
            .ok_or(wasm_error!("Game details not found"))?;

        match details {
            Details::Record(rec) => {
                if let Some(update) = rec.updates.last() {
                    current = update.action_address().clone();
                } else {
                    return Ok(rec.record);
                }
            }
            _ => return Err(wasm_error!("Unexpected details variant")),
        }
    }
}

/// ───────────────────── broadcast helper ──────────────────────
fn broadcast_to_opponents(game_id: &ActionHash, signal: &Signal) -> ExternResult<()> {
    // 1. load the *latest* Game entry
    let record = latest_record(game_id)?;
    let game: Game = record
        .entry()
        .to_app_option::<Game>()
        .map_err(|e| wasm_error!(e.to_string()))?
        .ok_or(wasm_error!("Malformed Game entry"))?;

    // 2. build recipient list (everyone except me)
    let me = agent_info()?.agent_latest_pubkey;
    let recipients = [&Some(game.player_1.clone()), &game.player_2]
        .iter()
        .filter_map(|o| o.as_ref())
        .filter(|pk| **pk != me)
        .cloned()
        .collect::<Vec<_>>();

    // 3. fire-and-forget
    for agent in recipients {
        let _ = call_remote(
            agent,
            "ping_2_pong",               
            "receive_remote_signal".into(),
            None,                       
            signal,              
        );
    }
    Ok(())
}

/// ───────────────────── externs used by UI ────────────────────
#[hdk_extern]
pub fn send_paddle_update(payload: PaddleUpdatePayload) -> ExternResult<()> {
    let signal = Signal::PaddleUpdate {
        game_id:  payload.game_id.clone(),
        player:   agent_info()?.agent_latest_pubkey,
        paddle_y: payload.paddle_y,
    };
    emit_signal(&signal)?;
    broadcast_to_opponents(&payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_game_abandoned_signal(payload: GameAbandonedPayload) -> ExternResult<()> {
    let abandoned_by_player = agent_info()?.agent_latest_pubkey;
    let signal = Signal::GameAbandoned { 
        game_id: payload.game_id.clone(),
        abandoned_by_player,
    };
        
    // Broadcast to the opponent
    broadcast_to_opponents(&payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_ball_update(payload: BallUpdatePayload) -> ExternResult<()> {
    let signal = Signal::BallUpdate {
        game_id: payload.game_id.clone(),
        ball_x:  payload.ball_x,
        ball_y:  payload.ball_y,
        ball_dx: payload.ball_dx,
        ball_dy: payload.ball_dy,
    };
    emit_signal(&signal)?;
    broadcast_to_opponents(&payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_score_update(payload: GameOverPayload) -> ExternResult<()> {
    let signal = Signal::ScoreUpdate {
        game_id: payload.game_id.clone(),
        score1:  payload.score1,
        score2:  payload.score2,
    };
    emit_signal(&signal)?;
    broadcast_to_opponents(&payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_game_over(payload: GameOverPayload) -> ExternResult<()> {
    let signal = Signal::GameOver {
        game_id: payload.game_id.clone(),
        winner:  payload.winner.clone(),
        score1:  payload.score1,
        score2:  payload.score2,
    };
    emit_signal(&signal)?;
    broadcast_to_opponents(&payload.game_id, &signal)
}
