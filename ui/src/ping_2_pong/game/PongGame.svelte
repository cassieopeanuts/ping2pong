<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from "svelte";
  import { getContext } from "svelte";
  import type { AppClient, HolochainError, ActionHash, AgentPubKey } from "@holochain/client";
  import { clientContext, type ClientContext } from "../../contexts";
  import type { Game } from "../ping_2_pong/types";

  const dispatch = createEventDispatcher();
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  export let game: Game;
  export let playerKey: AgentPubKey; // assume AgentPubKey type for the current player

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;

  // Initialize state from the game entry.
  let ballPosition = { x: game.ball_x, y: game.ball_y };
  let paddle1Position = { x: 20, y: game.player_1_paddle };
  let paddle2Position = { x: 760, y: game.player_2_paddle };

  let animationFrameId: number;
  let unsubscribeFromSignals: () => void;

  function handleKeyDown(e: KeyboardEvent) {
    if (playerKey === game.player_1) {
      if (e.key === "ArrowUp") {
        paddle1Position.y = Math.max(0, paddle1Position.y - 10);
        sendPaddleUpdate(paddle1Position.y);
      } else if (e.key === "ArrowDown") {
        paddle1Position.y = Math.min(600 - 100, paddle1Position.y + 10);
        sendPaddleUpdate(paddle1Position.y);
      }
    } else if (playerKey === game.player_2) {
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
      // We assume the updated game entry is passed in as only the changed paddle positions (and ball position)
      await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "update_paddle_position",
        payload: {
          original_game_hash: game.game_id,
          previous_game_hash: game.game_id, // in a real app, you’d want the latest hash here
          updated_game: {
            game_id: game.game_id,
            player_1_paddle: playerKey === game.player_1 ? newY : paddle1Position.y,
            player_2_paddle: playerKey === game.player_2 ? newY : paddle2Position.y,
            ball_x: ballPosition.x,
            ball_y: ballPosition.y,
          }
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
    ctx.beginPath();
    ctx.arc(ballPosition.x, ballPosition.y, 10, 0, 2 * Math.PI);
    ctx.fillStyle = "#fff";
    ctx.fill();
    ctx.closePath();
    ctx.fillStyle = "#fff";
    ctx.fillRect(paddle1Position.x, paddle1Position.y, 10, 100);
    ctx.fillRect(paddle2Position.x, paddle2Position.y, 10, 100);
    animationFrameId = requestAnimationFrame(draw);
  }

  function subscribeToGameSignals() {
    const unsubscribe = client.on("signal", (signal: any) => {
      // Check for our custom GameUpdate signal type.
      if (signal && signal.type === "GameUpdate" && signal.game_id === game.game_id) {
        if (signal.ball_x !== undefined && signal.ball_y !== undefined) {
          ballPosition = { x: signal.ball_x, y: signal.ball_y };
        }
        if (signal.paddle1 !== undefined) {
          paddle1Position.y = signal.paddle1;
        }
        if (signal.paddle2 !== undefined) {
          paddle2Position.y = signal.paddle2;
        }
        dispatch("game-updated", { game });
      }
    });
    return unsubscribe;
  }

  onMount(async () => {
    client = await appClientContext.getClient();
    ctx = canvas.getContext("2d");
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
