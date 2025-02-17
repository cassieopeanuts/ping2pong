<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { getContext } from "svelte";
  import type { AppClient, HolochainError, ActionHash, AgentPubKey } from "@holochain/client";
  import { clientContext, type ClientContext } from "../../contexts";
  import type { Game } from "../ping_2_pong/types";

  const dispatch = createEventDispatcher();

  // Props: the live game ID and the current player's public key.
  export let gameId: ActionHash;
  export let playerKey: AgentPubKey;

  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // Live game state (to be fetched from the DHT)
  let liveGame: Game | undefined;

  // Local state for rendering the game.
  let ballPosition = { x: 0, y: 0 };
  let paddle1Position = { x: 20, y: 250 };
  let paddle2Position = { x: 760, y: 250 };

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let animationFrameId: number;
  let unsubscribeFromSignals: () => void;

  async function fetchLiveGame() {
    try {
      const result = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_latest_game",
        payload: gameId,
      });
      console.log("Fetched game record:", result);
      // Check that the result has the expected structure.
      if (
        result &&
        result.entry &&
        result.entry.Present &&
        result.entry.Present.entry
      ) {
        liveGame = result.entry.Present.entry as Game;
        console.log("Deserialized live game:", liveGame);
        ballPosition = { x: liveGame.ball_x, y: liveGame.ball_y };
        paddle1Position = { x: 20, y: liveGame.player_1 ? liveGame.player_1_paddle : 250 };
        paddle2Position = { x: 760, y: liveGame.player_2 ? liveGame.player_2_paddle : 250 };
      } else {
        console.warn("Game record structure is not as expected:", result);
      }
    } catch (e) {
      console.error("Error fetching live game state:", e);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (!liveGame) return;
    // Check if current player is player1.
    if (playerKey.toString() === liveGame.player_1.toString()) {
      if (e.key === "ArrowUp") {
        paddle1Position.y = Math.max(0, paddle1Position.y - 10);
        sendPaddleUpdate(paddle1Position.y);
      } else if (e.key === "ArrowDown") {
        paddle1Position.y = Math.min(600 - 100, paddle1Position.y + 10);
        sendPaddleUpdate(paddle1Position.y);
      }
    }
    // Check if current player is player2.
    else if (liveGame.player_2 && playerKey.toString() === liveGame.player_2.toString()) {
      if (e.key === "ArrowUp") {
        paddle2Position.y = Math.max(0, paddle2Position.y - 10);
        sendPaddleUpdate(paddle2Position.y);
      } else if (e.key === "ArrowDown") {
        paddle2Position.y = Math.min(600 - 100, paddle2Position.y + 10);
        sendPaddleUpdate(paddle2Position.y);
      }
    }
  }

  async function sendPaddleUpdate(newY: number) {
    try {
      const updatedGame = {
        // We no longer send a game_id field.
        player_1_paddle: playerKey.toString() === liveGame?.player_1.toString() ? newY : paddle1Position.y,
        player_2_paddle: liveGame?.player_2 && playerKey.toString() === liveGame.player_2.toString() ? newY : paddle2Position.y,
        ball_x: ballPosition.x,
        ball_y: ballPosition.y,
      };

      await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "update_paddle_position",
        payload: {
          // For simplicity, using gameId as both original and previous.
          original_game_hash: gameId,
          previous_game_hash: gameId,
          updated_game: updatedGame,
        },
      });
    } catch (e) {
      console.error("Error updating paddle position:", (e as HolochainError).message);
    }
  }

  function draw() {
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Draw the ball.
    ctx.beginPath();
    ctx.arc(ballPosition.x, ballPosition.y, 10, 0, 2 * Math.PI);
    ctx.fillStyle = "#fff";
    ctx.fill();
    ctx.closePath();

    // Draw the paddles.
    ctx.fillStyle = "#fff";
    ctx.fillRect(paddle1Position.x, paddle1Position.y, 10, 100);
    ctx.fillRect(paddle2Position.x, paddle2Position.y, 10, 100);

    animationFrameId = requestAnimationFrame(draw);
  }

  function subscribeToGameSignals() {
    return client.on("signal", (signal: any) => {
      if (signal && signal.type === "GameUpdate" && signal.game_id === gameId) {
        if (signal.ball_x !== undefined && signal.ball_y !== undefined) {
          ballPosition = { x: signal.ball_x, y: signal.ball_y };
        }
        if (signal.paddle1 !== undefined) {
          paddle1Position.y = signal.paddle1;
        }
        if (signal.paddle2 !== undefined) {
          paddle2Position.y = signal.paddle2;
        }
      }
    });
  }

  onMount(async () => {
    client = await appClientContext.getClient();
    await fetchLiveGame();
    draw();
    window.addEventListener("keydown", handleKeyDown);
    unsubscribeFromSignals = subscribeToGameSignals();
  });

  onDestroy(() => {
    cancelAnimationFrame(animationFrameId);
    window.removeEventListener("keydown", handleKeyDown);
    if (unsubscribeFromSignals) unsubscribeFromSignals();
  });
</script>

<div class="game-window">
  <!-- Display current players' public keys -->
  {#if liveGame}
    <div class="players-info">
      <div class="player player1">
        {liveGame.player_1 ? liveGame.player_1.toString() : "Unknown"}
      </div>
      <div class="player player2">
        {liveGame.player_2 ? liveGame.player_2.toString() : "Waiting for player..."}
      </div>
    </div>
  {/if}
  <canvas bind:this={canvas} width="800" height="600"></canvas>
</div>

<style>
  .game-window {
    position: relative;
  }
  .players-info {
    position: absolute;
    top: 10px;
    width: 100%;
    display: flex;
    justify-content: space-between;
    padding: 0 20px;
    color: orange;
    font-size: 1rem;
    font-weight: bold;
    z-index: 1;
  }
  .player {
    /* Additional styling if needed */
  }
  canvas {
    background-color: #000;
    display: block;
    margin: 0 auto;
    border: 2px solid #646cff;
  }
</style>
