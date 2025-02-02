<script lang="ts">
    import { onMount, onDestroy, createEventDispatcher } from "svelte";
    import { getContext } from "svelte";
    import type { AppClient, HolochainError, SignedActionHashed } from "@holochain/client";
    import { clientContext, type ClientContext } from "../../contexts";
    import type { Game, GameStatus } from "../ping_2_pong/types";
  
    const dispatch = createEventDispatcher();
    let client: AppClient;
    // Use the type parameter so TypeScript knows that getClient() exists.
    const appClientContext = getContext<ClientContext>(clientContext);
  
    // Passed-in props: the current game state and the current player's key.
    export let game: Game;
    export let playerKey: string;
  
    let canvas: HTMLCanvasElement;
    let ctx: CanvasRenderingContext2D;
  
    // Example state values (to be updated via signals)
    let ballPosition = { x: 400, y: 300 };
    let paddle1Position = { x: 20, y: 250 };
    let paddle2Position = { x: 760, y: 250 };
  
    let animationFrameId: number;
    let unsubscribeFromSignals: () => void;
  
    // Keyboard input to move the paddle.
    function handleKeyDown(e: KeyboardEvent) {
      if (playerKey === game.player_1.toString()) {
        // For Player 1 (left paddle)
        if (e.key === "ArrowUp") {
          paddle1Position.y = Math.max(0, paddle1Position.y - 10);
          sendPaddleUpdate(paddle1Position.y);
        } else if (e.key === "ArrowDown") {
          paddle1Position.y = Math.min(600 - 100, paddle1Position.y + 10);
          sendPaddleUpdate(paddle1Position.y);
        }
      } else if (playerKey === game.player_2.toString()) {
        // For Player 2 (right paddle)
        if (e.key === "ArrowUp") {
          paddle2Position.y = Math.max(0, paddle2Position.y - 10);
          sendPaddleUpdate(paddle2Position.y);
        } else if (e.key === "ArrowDown") {
          paddle2Position.y = Math.min(600 - 100, paddle2Position.y + 10);
          sendPaddleUpdate(paddle2Position.y);
        }
      }
    }
  
    // Send the updated paddle position to the backend.
    async function sendPaddleUpdate(newY: number) {
      try {
        await client.callZome({
          cap_secret: null,
          role_name: "ping_2_pong",
          zome_name: "ping_2_pong",
          fn_name: "update_paddle_position",
          payload: { game_id: game.game_id, player: playerKey, new_position: newY },
        });
      } catch (e) {
        console.error("Error updating paddle position:", (e as HolochainError).message);
      }
    }
  
    // Main drawing function for the canvas.
    function draw() {
      if (!ctx) return;
      // Clear canvas.
      ctx.clearRect(0, 0, canvas.width, canvas.height);
  
      // Draw background.
      ctx.fillStyle = "#000";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
  
      // Draw ball.
      ctx.beginPath();
      ctx.arc(ballPosition.x, ballPosition.y, 10, 0, 2 * Math.PI);
      ctx.fillStyle = "#fff";
      ctx.fill();
      ctx.closePath();
  
      // Draw left paddle (Player 1).
      ctx.fillStyle = "#fff";
      ctx.fillRect(paddle1Position.x, paddle1Position.y, 10, 100);
  
      // Draw right paddle (Player 2).
      ctx.fillStyle = "#fff";
      ctx.fillRect(paddle2Position.x, paddle2Position.y, 10, 100);
  
      // Request next frame.
      animationFrameId = requestAnimationFrame(draw);
    }
  
    // Subscribe to signals from the conductor.
    function subscribeToGameSignals() {
  // Use the client.on method to subscribe to signals.
  const unsubscribe = client.on("signal", (signal: any) => {
    // Log the received signal (for debugging).
    console.log("Received signal:", signal);
    
    // Check if the signal is a game update and matches our current game.
    // Adjust the filtering logic based on the signal structure emitted by your DNA.
    if (signal && signal.type === "GameUpdate" && signal.game_id === game.game_id) {
      // Update ball position if provided.
      if (signal.ball) {
        ballPosition = { ...signal.ball };
      }
      // Update paddle positions if provided.
      if (signal.paddle1 !== undefined) {
        paddle1Position.y = signal.paddle1;
      }
      if (signal.paddle2 !== undefined) {
        paddle2Position.y = signal.paddle2;
      }
      // Optionally, dispatch an event or update additional game state.
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
      // Subscribe to ephemeral signals.
      unsubscribeFromSignals = subscribeToGameSignals();
    });
  
    onDestroy(() => {
      cancelAnimationFrame(animationFrameId);
      window.removeEventListener("keydown", handleKeyDown);
      if (unsubscribeFromSignals) {
        unsubscribeFromSignals();
      }
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
  