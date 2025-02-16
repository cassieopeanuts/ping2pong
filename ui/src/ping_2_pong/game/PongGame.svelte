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
  let liveGame: Game;

  // Local state for rendering the game.
  let ballPosition = { x: 0, y: 0 };
  let paddle1Position = { x: 20, y: 250 };
  let paddle2Position = { x: 760, y: 250 };

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let animationFrameId: number;
  let unsubscribeFromSignals: () => void;

  // Fetch the live game state from the DHT.
  async function fetchLiveGame() {
    try {
      const result = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_latest_game",
        payload: gameId,
      });
      if (result) {
        // Extract the game entry from the record.
        liveGame = (result.entry as any)?.Present?.entry as Game;
        if (liveGame) {
          ballPosition = { x: liveGame.ball_x, y: liveGame.ball_y };
          paddle1Position = { x: 20, y: liveGame.player_1_paddle };
          paddle2Position = { x: 760, y: liveGame.player_2_paddle };
        }
      }
    } catch (e) {
      console.error("Error fetching live game state:", e);
    }
  }

  // Handle keyboard input to move the paddle.
  function handleKeyDown(e: KeyboardEvent) {
    if (!liveGame) return;
    // Check if the current player is player_1.
    if (playerKey === liveGame.player_1) {
      if (e.key === "ArrowUp") {
        paddle1Position.y = Math.max(0, paddle1Position.y - 10);
        sendPaddleUpdate(paddle1Position.y);
      } else if (e.key === "ArrowDown") {
        paddle1Position.y = Math.min(600 - 100, paddle1Position.y + 10);
        sendPaddleUpdate(paddle1Position.y);
      }
    }
    // Otherwise, if current player is player_2.
    else if (playerKey === liveGame.player_2) {
      if (e.key === "ArrowUp") {
        paddle2Position.y = Math.max(0, paddle2Position.y - 10);
        sendPaddleUpdate(paddle2Position.y);
      } else if (e.key === "ArrowDown") {
        paddle2Position.y = Math.min(600 - 100, paddle2Position.y + 10);
        sendPaddleUpdate(paddle2Position.y);
      }
    }
  }

  // Send the updated paddle position to the coordinator.
  async function sendPaddleUpdate(newY: number) {
    try {
      // Build updated game data.
      const updatedGame = {
        game_id: liveGame.game_id,
        player_1_paddle: playerKey === liveGame.player_1 ? newY : paddle1Position.y,
        player_2_paddle: playerKey === liveGame.player_2 ? newY : paddle2Position.y,
        ball_x: ballPosition.x,
        ball_y: ballPosition.y,
      };

      await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "update_paddle_position",
        payload: {
          // For simplicity, using game_id as both original and previous hash.
          // In production, track the latest update hash.
          original_game_hash: liveGame.game_id,
          previous_game_hash: liveGame.game_id,
          updated_game: updatedGame,
        },
      });
    } catch (e) {
      console.error("Error updating paddle position:", (e as HolochainError).message);
    }
  }

  // Draw the game on the canvas.
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

  // Subscribe to game signals to update live state.
  function subscribeToGameSignals() {
    return client.on("signal", (signal: any) => {
      if (signal && signal.type === "GameUpdate" && signal.game_id === liveGame.game_id) {
        if (signal.ball_x !== undefined && signal.ball_y !== undefined) {
          ballPosition = { x: signal.ball_x, y: signal.ball_y };
        }
        if (signal.paddle1 !== undefined) {
          paddle1Position.y = signal.paddle1;
        }
        if (signal.paddle2 !== undefined) {
          paddle2Position.y = signal.paddle2;
        }
        // Optionally update the liveGame state as well.
        liveGame = {
          ...liveGame,
          ball_x: signal.ball_x,
          ball_y: signal.ball_y,
          player_1_paddle: signal.paddle1,
          player_2_paddle: signal.paddle2,
        };
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

<canvas bind:this={canvas} width="800" height="600"></canvas>

<style>
  canvas {
    background-color: #000;
    display: block;
    margin: 0 auto;
    border: 2px solid #646cff;
  }
</style>
