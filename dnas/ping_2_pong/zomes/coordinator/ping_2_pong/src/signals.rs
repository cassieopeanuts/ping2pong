// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/signals.rs
use hdk::prelude::*;
use crate::Signal;

/// ───────────────────────── init helper ─────────────────────────
pub fn grant_remote_signal_cap() -> ExternResult<()> {
    let grant = CapGrantEntry {
        tag: "remote-signal".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::All,
    };
    let _ = create_cap_grant(grant);
    Ok(())
}

/// ──────────────────────── local re-emit ───────────────────────
#[hdk_extern]
pub fn receive_remote_signal(input: ExternIO) -> ExternResult<()> {
    let _ = grant_remote_signal_cap();
    let signal: Signal = input.decode().map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;
    emit_signal(&signal)
}

/// ─────────────────────── payload structs ──────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaddleUpdatePayload {
    pub game_id:   ActionHash,
    pub recipient: Option<AgentPubKey>,
    pub paddle_y:  i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaddleHitPayload {
    pub game_id:   ActionHash,
    pub recipient: Option<AgentPubKey>,
    pub ball_y:    i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BallUpdatePayload {
    pub game_id:   ActionHash,
    pub recipient: Option<AgentPubKey>,
    pub ball_x:    i32,
    pub ball_y:    i32,
    pub ball_dx:   i32,
    pub ball_dy:   i32,
    pub score1:    Option<u32>,
    pub score2:    Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameOverPayload {
    pub game_id:   ActionHash,
    pub recipient: Option<AgentPubKey>,
    pub winner:    Option<AgentPubKey>,
    pub score1:    u32,
    pub score2:    u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameAbandonedPayload {
    pub game_id:   ActionHash,
    pub recipient: Option<AgentPubKey>,
}

/// ───────────────────── broadcast helper ──────────────────────
fn broadcast_signal(recipient: Option<AgentPubKey>, _game_id: &ActionHash, signal: &Signal) -> ExternResult<()> {
    let signal_io = ExternIO::encode(signal).map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?;

    if let Some(target) = recipient {
        let me = agent_info()?.agent_initial_pubkey;
        if target != me {
            let _ = call_remote(
                target,
                "ping_2_pong",
                "receive_remote_signal".into(),
                None,
                signal_io,
            );
        }
    }
    Ok(())
}

/// ───────────────────── externs used by UI ────────────────────
#[hdk_extern]
pub fn send_paddle_update(payload: PaddleUpdatePayload) -> ExternResult<()> {
    let signal = Signal::PaddleUpdate {
        game_id:  payload.game_id.clone(),
        player:   agent_info()?.agent_initial_pubkey,
        paddle_y: payload.paddle_y,
    };
    emit_signal(&signal)?;
    broadcast_signal(payload.recipient, &payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_paddle_hit(payload: PaddleHitPayload) -> ExternResult<()> {
    let signal = Signal::PaddleHit {
        game_id:  payload.game_id.clone(),
        player:   agent_info()?.agent_initial_pubkey,
        ball_y:   payload.ball_y,
    };
    emit_signal(&signal)?;
    broadcast_signal(payload.recipient, &payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_game_abandoned_signal(payload: GameAbandonedPayload) -> ExternResult<()> {
    let abandoned_by_player = agent_info()?.agent_initial_pubkey;
    let signal = Signal::GameAbandoned {
        game_id: payload.game_id.clone(),
        abandoned_by_player,
    };
    
    broadcast_signal(payload.recipient, &payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_ball_update(payload: BallUpdatePayload) -> ExternResult<()> {
    let signal = Signal::BallUpdate {
        game_id: payload.game_id.clone(),
        ball_x:  payload.ball_x,
        ball_y:  payload.ball_y,
        ball_dx: payload.ball_dx,
        ball_dy: payload.ball_dy,
        score1:  payload.score1,
        score2:  payload.score2,
    };
    emit_signal(&signal)?;
    broadcast_signal(payload.recipient, &payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_score_update(payload: GameOverPayload) -> ExternResult<()> {
    let signal = Signal::ScoreUpdate {
        game_id: payload.game_id.clone(),
        score1:  payload.score1,
        score2:  payload.score2,
    };
    emit_signal(&signal)?;
    broadcast_signal(payload.recipient, &payload.game_id, &signal)
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
    broadcast_signal(payload.recipient, &payload.game_id, &signal)
}
