<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getContext } from "svelte";
  import type { AppClient, ActionHash, AgentPubKey } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  import { clientContext, type ClientContext } from "../../contexts";
  import type { Game } from "../ping_2_pong/types";

  export let gameId: ActionHash;
  export let playerKey: AgentPubKey;

  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  const CANVAS_WIDTH = 800;
  const CANVAS_HEIGHT = 600;
  const PADDLE_WIDTH = 10;
  const PADDLE_HEIGHT = 100;
  const BALL_RADIUS = 10;

  let liveGame: Game | undefined;
  let isPlayer1 = false;
  let isPlayer2 = false;
  let paddle1Y = CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2;
  let paddle2Y = CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2;
  let ball = { x: CANVAS_WIDTH / 2, y: CANVAS_HEIGHT / 2, dx: 5, dy: 5 };
  let score = { player1: 0, player2: 0 };

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let animationFrameId: number;
  let unsubscribeFromSignals: () => void;

  let lastPaddleUpdate = 0;
  let lastBallUpdate = 0;
  const UPDATE_INTERVAL = 200;

  async function fetchGameState() {
    try {
      const result = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_latest_game",
        payload: gameId,
      });
      console.log("Fetched game record:", result);
      if (result && result.entry && result.entry.Present && result.entry.Present.entry) {
        const game = result.entry.Present.entry as Game;
        console.log("Deserialized game:", game);
        if (!game.player_1) {
          console.error("Game record is missing player_1.");
          return;
        }
        liveGame = game;
        isPlayer1 = encodeHashToBase64(playerKey) === encodeHashToBase64(game.player_1);
        isPlayer2 = game.player_2 ? (encodeHashToBase64(playerKey) === encodeHashToBase64(game.player_2)) : false;
        paddle1Y = game.player_1_paddle;
        paddle2Y = game.player_2_paddle || (CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2);
        ball.x = game.ball_x;
        ball.y = game.ball_y;
      }
    } catch (e) {
      console.error("Error fetching game state:", e);
    }
  }

  function liveGameAvailable(): boolean {
    return liveGame !== undefined;
  }

  function handleKeyDown(e: KeyboardEvent) {
    const paddleSpeed = 10;
    if (!liveGameAvailable()) return;
    if (isPlayer1) {
      if (e.key === "ArrowUp") {
        paddle1Y = Math.max(0, paddle1Y - paddleSpeed);
        sendPaddleUpdate();
      } else if (e.key === "ArrowDown") {
        paddle1Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle1Y + paddleSpeed);
        sendPaddleUpdate();
      }
    } else if (isPlayer2) {
      if (e.key === "ArrowUp") {
        paddle2Y = Math.max(0, paddle2Y - paddleSpeed);
        sendPaddleUpdate();
      } else if (e.key === "ArrowDown") {
        paddle2Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle2Y + paddleSpeed);
        sendPaddleUpdate();
      }
    }
  }

  async function sendPaddleUpdate() {
    const now = Date.now();
    if (now - lastPaddleUpdate < UPDATE_INTERVAL) return;
    lastPaddleUpdate = now;
    const signal = {
      type: "PaddleUpdate",
      game_id: gameId,
      player: playerKey,
      paddle_y: isPlayer1 ? paddle1Y : paddle2Y,
    };
    try {
      await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "send_signal",
        payload: signal,
      });
    } catch (e) {
      console.error("Error sending paddle update signal:", e);
    }
  }

  async function sendBallUpdate() {
    if (!isPlayer1) return;
    const now = Date.now();
    if (now - lastBallUpdate < UPDATE_INTERVAL) return;
    lastBallUpdate = now;
    const signal = {
      type: "BallUpdate",
      game_id: gameId,
      ball_x: ball.x,
      ball_y: ball.y,
      ball_dx: ball.dx,
      ball_dy: ball.dy,
    };
    try {
      await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "send_signal",
        payload: signal,
      });
    } catch (e) {
      console.error("Error sending ball update signal:", e);
    }
  }

  function subscribeToGameSignals() {
    return client.on("signal", (signal: any) => {
      if (signal.game_id?.toString() === gameId.toString()) {
        if (signal.type === "PaddleUpdate") {
          if (signal.player.toString() !== encodeHashToBase64(playerKey)) {
            if (isPlayer1) {
              paddle2Y = signal.paddle_y;
            } else if (isPlayer2) {
              paddle1Y = signal.paddle_y;
            }
          }
        } else if (signal.type === "BallUpdate" && !isPlayer1) {
          ball.x = signal.ball_x;
          ball.y = signal.ball_y;
          ball.dx = signal.ball_dx;
          ball.dy = signal.ball_dy;
        }
      }
    });
  }

  function updateBall() {
    if (!isPlayer1) return;
    ball.x += ball.dx;
    ball.y += ball.dy;
    if (ball.y + BALL_RADIUS > CANVAS_HEIGHT || ball.y - BALL_RADIUS < 0) {
      ball.dy = -ball.dy;
    }
    if (
      ball.x - BALL_RADIUS < PADDLE_WIDTH &&
      ball.y > paddle1Y &&
      ball.y < paddle1Y + PADDLE_HEIGHT
    ) {
      ball.dx = -ball.dx;
      ball.x = PADDLE_WIDTH + BALL_RADIUS;
    }
    if (
      ball.x + BALL_RADIUS > CANVAS_WIDTH - PADDLE_WIDTH &&
      ball.y > paddle2Y &&
      ball.y < paddle2Y + PADDLE_HEIGHT
    ) {
      ball.dx = -ball.dx;
      ball.x = CANVAS_WIDTH - PADDLE_WIDTH - BALL_RADIUS;
    }
    if (ball.x < 0 || ball.x > CANVAS_WIDTH) {
      ball.x = CANVAS_WIDTH / 2;
      ball.y = CANVAS_HEIGHT / 2;
      ball.dx = 5 * (Math.random() > 0.5 ? 1 : -1);
      ball.dy = 5 * (Math.random() > 0.5 ? 1 : -1);
    }
  }

  function draw() {
    if (!ctx) return;
    ctx.clearRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
    ctx.fillStyle = "#000";
    ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

    ctx.beginPath();
    ctx.arc(ball.x, ball.y, BALL_RADIUS, 0, 2 * Math.PI);
    ctx.fillStyle = "#fff";
    ctx.fill();
    ctx.closePath();

    ctx.fillStyle = "#fff";
    ctx.fillRect(0, paddle1Y, PADDLE_WIDTH, PADDLE_HEIGHT);
    ctx.fillRect(CANVAS_WIDTH - PADDLE_WIDTH, paddle2Y, PADDLE_WIDTH, PADDLE_HEIGHT);

    ctx.font = "30px Arial";
    ctx.fillText(score.player1.toString(), CANVAS_WIDTH / 4, 50);
    ctx.fillText(score.player2.toString(), (3 * CANVAS_WIDTH) / 4, 50);

    updateBall();
    sendBallUpdate();

    animationFrameId = requestAnimationFrame(draw);
  }

  onMount(async () => {
    client = await appClientContext.getClient();
    await fetchGameState();
    ctx = canvas.getContext("2d")!;
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
  <div class="players-info">
    <div class="player player1">
      {#if liveGameAvailable()}
        {encodeHashToBase64(liveGame.player_1)}
      {:else}
        Loading player info...
      {/if}
    </div>
    <div class="player player2">
      {#if liveGame && liveGame.player_2}
        {encodeHashToBase64(liveGame.player_2)}
      {:else}
        Waiting for player...
      {/if}
    </div>
  </div>
  <canvas bind:this={canvas} width={CANVAS_WIDTH} height={CANVAS_HEIGHT}></canvas>
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
