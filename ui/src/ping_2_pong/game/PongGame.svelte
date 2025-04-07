<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getContext } from "svelte";
  import type { AppClient, ActionHash, AgentPubKey, Record } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  import { clientContext, type ClientContext } from "../../contexts";
  import type { Game, Score, GameStatus } from "../ping_2_pong/types"; // Import GameStatus  import { currentRoute } from "../../stores/routeStore"; // Import for navigation

  export let gameId: ActionHash;
  export let playerKey: AgentPubKey; // This is passed as AgentPubKey object now

  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  const CANVAS_WIDTH = 800;
  const CANVAS_HEIGHT = 600;
  const PADDLE_WIDTH = 10;
  const PADDLE_HEIGHT = 100;
  const BALL_RADIUS = 10;
  const WINNING_SCORE = 10; // Define winning score

  let gameRecord: Record | undefined; // Store the full record for updates
  let liveGame: Game | undefined;
  let isPlayer1 = false;
  let isPlayer2 = false;
  let paddle1Y = CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2;
  let paddle2Y = CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2;
  let ball = { x: CANVAS_WIDTH / 2, y: CANVAS_HEIGHT / 2, dx: 5, dy: 5 };
  let score = { player1: 0, player2: 0 };
  let gameOver = false; // Game state flag
  let winner: AgentPubKey | null = null; // Store winner's pubkey

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let animationFrameId: number;
  let unsubscribeFromSignals: (() => void) | undefined;

  let lastPaddleUpdate = 0;
  let lastBallUpdate = 0;
  const UPDATE_INTERVAL = 50; // Send updates more frequently (adjust as needed)

  async function fetchGameState() {
    // Ensure client and gameId are available
    if (!client || !gameId) {
        console.error("Client or gameId not available for fetchGameState");
        return;
    }
    try {
      console.log("Fetching game state for:", encodeHashToBase64(gameId));
      const result: Record | null = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_latest_game",
        payload: gameId,
      });

      // Check if a record was returned
      if (result) {
        gameRecord = result; // Store the full record

        // FIX: Access entry data correctly and cast
        // Assuming the client deserializes the entry based on its type
        const entryData = result.entry;

        // Check if entry data exists before trying to use it
        if (entryData) {
            // Cast the entry data to the Game interface.
            // This assumes the field names match (snake_case from Rust matches TS)
            // and types are compatible (AgentPubKey, number, etc.)
            // Add type assertion if confident, or add runtime checks if needed.
            const game = entryData as unknown as Game; // Cast to Game

            console.log("Fetched game data:", game);

            // Validate essential data
            if (!game.player_1) { // Check property exists on the casted object
              console.error("Fetched game data is missing player_1.");
              // Maybe set an error state or navigate back
              return;
            }

            liveGame = game; // Assign successfully casted game data

            const myPubKeyB64 = encodeHashToBase64(playerKey);
            isPlayer1 = encodeHashToBase64(game.player_1) === myPubKeyB64;
            // Ensure player_2 check handles null correctly
            isPlayer2 = !!game.player_2 && (encodeHashToBase64(game.player_2) === myPubKeyB64);

            console.log(`Is Player 1: ${isPlayer1}, Is Player 2: ${isPlayer2}`);

            // Initialize positions only if score is 0 (first load)
            if (score.player1 === 0 && score.player2 === 0) {
                // Check properties exist before accessing
                paddle1Y = game.player_1_paddle ?? (CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2);
                paddle2Y = game.player_2_paddle ?? (CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2);
                ball.x = game.ball_x ?? (CANVAS_WIDTH / 2);
                ball.y = game.ball_y ?? (CANVAS_HEIGHT / 2);
                ball.dx = 5 * (Math.random() > 0.5 ? 1 : -1);
                ball.dy = 5 * (Math.random() > 0.5 ? 1 : -1);
            }

            // Check game status exists before accessing type
            if (liveGame.game_status?.type === 'Finished') {
                console.log("Game already finished on load.");
                gameOver = true;
                // TODO: Fetch score/winner if needed
            }

        } else {
            console.error("Fetched record is missing entry data for gameId:", encodeHashToBase64(gameId));
            gameRecord = undefined; // Clear record if entry is missing
            liveGame = undefined;
        }

      } else {
          console.error("Failed to fetch record for gameId:", encodeHashToBase64(gameId));
          gameRecord = undefined; // Clear record if fetch failed
          liveGame = undefined;
          // Handle error state - maybe navigate back?
      }
    } catch (e) {
      console.error("Error fetching game state:", e);
       gameRecord = undefined; // Clear record on error
       liveGame = undefined;
    }
  }

  function liveGameAvailable(): boolean {
    return liveGame !== undefined;
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (gameOver || !liveGameAvailable()) return; // Stop input if game over

    const paddleSpeed = 15; // Slightly faster paddle
    let moved = false;

    if (isPlayer1) {
      if (e.key === "ArrowUp" || e.key === "w" || e.key === "W") {
        paddle1Y = Math.max(0, paddle1Y - paddleSpeed);
        moved = true;
      } else if (e.key === "ArrowDown" || e.key === "s" || e.key === "S") {
        paddle1Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle1Y + paddleSpeed);
        moved = true;
      }
    } else if (isPlayer2) {
      if (e.key === "ArrowUp" || e.key === "w" || e.key === "W") {
        paddle2Y = Math.max(0, paddle2Y - paddleSpeed);
        moved = true;
      } else if (e.key === "ArrowDown" || e.key === "s" || e.key === "S") {
        paddle2Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle2Y + paddleSpeed);
        moved = true;
      }
    }
    if (moved) {
        sendPaddleUpdate(); // Send update if paddle moved
    }
  }

  async function sendPaddleUpdate() {
    if (gameOver || !client || !liveGameAvailable()) return;

    const now = Date.now();
    if (now - lastPaddleUpdate < UPDATE_INTERVAL) return; // Throttle updates
    lastPaddleUpdate = now;

    // Send only own paddle position
    const signal = {
      type: "PaddleUpdate",
      game_id: gameId,
      player: playerKey, // Send AgentPubKey object
      paddle_y: isPlayer1 ? paddle1Y : paddle2Y,
    };
    try {
      // Use send_signal for ephemeral updates
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
    if (gameOver || !isPlayer1 || !client || !liveGameAvailable()) return; // Only P1 sends ball updates

    const now = Date.now();
    if (now - lastBallUpdate < UPDATE_INTERVAL) return; // Throttle updates
    lastBallUpdate = now;

    const signal = {
      type: "BallUpdate",
      game_id: gameId,
      ball_x: ball.x,
      ball_y: ball.y,
      ball_dx: ball.dx, // Include velocity for smoother prediction?
      ball_dy: ball.dy,
    };
    try {
       // Use send_signal for ephemeral updates
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
    if (!client) return undefined;
    return client.on("signal", (signalPayload: any) => {
        if (gameOver) return; // Ignore signals if game locally over

        // Check if signal is for this game
        if (!signalPayload.game_id || encodeHashToBase64(signalPayload.game_id) !== encodeHashToBase64(gameId)) {
            return;
        }

        const myPubKeyB64 = encodeHashToBase64(playerKey);

        if (signalPayload.type === "PaddleUpdate") {
          // Ignore signals about own paddle
          if (signalPayload.player && encodeHashToBase64(signalPayload.player) !== myPubKeyB64) {
             // Update opponent's paddle
             if (isPlayer1) {
                 paddle2Y = signalPayload.paddle_y;
             } else if (isPlayer2) {
                 paddle1Y = signalPayload.paddle_y;
             }
          }
        } else if (signalPayload.type === "BallUpdate") {
             // Only Player 2 should listen to Player 1's ball updates
             if (!isPlayer1) {
                 ball.x = signalPayload.ball_x;
                 ball.y = signalPayload.ball_y;
                 ball.dx = signalPayload.ball_dx;
                 ball.dy = signalPayload.ball_dy;
             }
        } else if (signalPayload.type === "GameOver") { // Optional: Signal for game over event
             console.log("Received GameOver signal:", signalPayload);
             if (!gameOver) { // Check if already over locally
                handleRemoteGameOver(signalPayload.winner);
             }
        }
    });
  }

  // Only Player 1 runs the main physics and scoring
  function updateBallAndScore() {
    if (gameOver || !isPlayer1 || !liveGame) return; // Only P1 updates ball/score

    ball.x += ball.dx;
    ball.y += ball.dy;

    // --- Ball Collision with Top/Bottom Walls ---
    if (ball.y + BALL_RADIUS > CANVAS_HEIGHT || ball.y - BALL_RADIUS < 0) {
      ball.dy = -ball.dy;
      // Clamp position to prevent sticking
      ball.y = Math.max(BALL_RADIUS, Math.min(CANVAS_HEIGHT - BALL_RADIUS, ball.y));
    }

    // --- Ball Collision with Paddles ---
    let hitPaddle = false;
    // Collision with Player 1 Paddle (Left)
    if (ball.dx < 0 && // Moving left
        ball.x - BALL_RADIUS < PADDLE_WIDTH && // Within paddle x-zone
        ball.x - BALL_RADIUS > 0 && // Not past the paddle edge yet
        ball.y > paddle1Y && ball.y < paddle1Y + PADDLE_HEIGHT // Within paddle y-zone
       )
    {
        ball.dx = -ball.dx * 1.05; // Reverse direction and slightly increase speed
        ball.x = PADDLE_WIDTH + BALL_RADIUS; // Adjust position to avoid sticking
        // Optional: Add vertical deflection based on where it hits paddle
        let deltaY = ball.y - (paddle1Y + PADDLE_HEIGHT / 2);
        ball.dy = deltaY * 0.35; // Adjust multiplier for desired effect
        hitPaddle = true;
    }
    // Collision with Player 2 Paddle (Right)
    else if (ball.dx > 0 && // Moving right
        ball.x + BALL_RADIUS > CANVAS_WIDTH - PADDLE_WIDTH && // Within paddle x-zone
        ball.x + BALL_RADIUS < CANVAS_WIDTH && // Not past the paddle edge yet
        ball.y > paddle2Y && ball.y < paddle2Y + PADDLE_HEIGHT // Within paddle y-zone
       )
    {
        ball.dx = -ball.dx * 1.05; // Reverse direction and slightly increase speed
        ball.x = CANVAS_WIDTH - PADDLE_WIDTH - BALL_RADIUS; // Adjust position
        // Optional: Add vertical deflection
        let deltaY = ball.y - (paddle2Y + PADDLE_HEIGHT / 2);
        ball.dy = deltaY * 0.35;
        hitPaddle = true;
    }

    // --- Scoring ---
    let scored = false;
    if (ball.x + BALL_RADIUS < 0) { // Player 2 scores
      score.player2++;
      console.log(`Score: ${score.player1} - ${score.player2}`);
      scored = true;
    } else if (ball.x - BALL_RADIUS > CANVAS_WIDTH) { // Player 1 scores
      score.player1++;
       console.log(`Score: ${score.player1} - ${score.player2}`);
      scored = true;
    }

    // --- Reset Ball After Score ---
    if (scored) {
        // Check for winner BEFORE resetting ball
        if (score.player1 >= WINNING_SCORE || score.player2 >= WINNING_SCORE) {
            winner = score.player1 >= WINNING_SCORE ? liveGame.player_1 : liveGame.player_2;
            gameOver = true;
            console.log("Game Over! Winner:", truncatePubkey(winner!));
            // Call function to save results etc.
            handleLocalGameOver();
            // Don't reset ball visually, just stop updates
        } else {
             // Reset ball to center, random direction
            ball.x = CANVAS_WIDTH / 2;
            ball.y = CANVAS_HEIGHT / 2;
            ball.dx = 5 * (score.player1 > score.player2 ? -1 : 1); // Serve towards loser
            ball.dy = 5 * (Math.random() > 0.5 ? 1 : -1);
            // Ensure ball update is sent immediately after reset
            lastBallUpdate = 0; // Force next update
            sendBallUpdate();
        }
    } else if (hitPaddle) {
         // If paddle hit, ensure ball update is sent
         lastBallUpdate = 0; // Force next update
         sendBallUpdate();
    } else {
        // If no score or hit, still send regular updates
        sendBallUpdate();
    }
  }

  // --- Game Over Handling ---
  async function handleLocalGameOver() {
      if (!liveGame || !gameRecord) return;
      console.log("Handling local game over...");
      // Stop animation loop handled by check in draw()
      // Cancel signal subscription? Or keep listening for opponent state? Let's keep listening for now.

       // 1. Update Game Status to Finished
       try {
            const latestGameState: Game = Game.deserialize(gameRecord.entry.Present.entry); // Use current state
            const finishedGameState: Game = {
                ...latestGameState,
                game_status: { type: 'Finished' },
                // Optional: Snapshot final paddle/ball pos if integrity rule allows
                 player_1_paddle: paddle1Y,
                 player_2_paddle: paddle2Y,
                 ball_x: ball.x,
                 ball_y: ball.y,
            };

            const updatePayload: UpdateGameInput = {
                 original_game_hash: gameRecord.signed_action.hashed.hash, // Original create hash
                 previous_game_hash: gameRecord.signed_action.hashed.hash, // Latest known action hash (could be different if game was updated before) - TODO: This needs careful handling if game updates were possible before finishing. Assume original = previous for now.
                 updated_game: finishedGameState,
            };
            console.log("Updating game status to Finished...");
            await client.callZome({
                 cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
                 fn_name: "update_game", payload: updatePayload,
            });
            console.log("Game status updated.");
       } catch (e) {
            console.error("Error updating game status:", e);
       }

       // 2. Save Scores
       try {
           const score1Payload: Score = {
               game_id: gameRecord.signed_action.hashed.hash,
               player: liveGame.player_1,
               player_points: score.player1,
               created_at: Date.now() // Coordinator might overwrite this
           };
            const score2Payload: Score = {
               game_id: gameRecord.signed_action.hashed.hash,
               player: liveGame.player_2!, // Should exist if game is over
               player_points: score.player2,
               created_at: Date.now()
           };
           console.log("Saving scores...");
           await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "create_score", payload: score1Payload });
           await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "create_score", payload: score2Payload });
           console.log("Scores saved.");
       } catch (e) {
           console.error("Error saving scores:", e);
       }

       // 3. (Optional) Send GameOver signal to ensure opponent knows
       try {
           const gameOverSignal = {
               type: "GameOver",
               game_id: gameId,
               winner: winner, // Send winner pubkey
               score1: score.player1,
               score2: score.player2,
           };
           await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "send_signal", payload: gameOverSignal });
           console.log("GameOver signal sent.");
       } catch(e) {
           console.error("Error sending GameOver signal:", e);
       }

      // 4. (Later) Save Statistics
      // await saveStatistics();
  }

  // Handle game over triggered by remote signal (if P2)
  function handleRemoteGameOver(remoteWinner: AgentPubKey | null) {
        if (gameOver) return; // Already over locally
        console.log("Handling remote game over signal...");
        gameOver = true;
        winner = remoteWinner;
        // Stop local updates if any were running
        // UI should update based on gameOver flag
  }

  // Function to navigate back
  function goBackToLobby() {
      currentRoute.set("dashboard");
  }

  // Main draw loop
  function draw() {
    if (!ctx) return animationFrameId = requestAnimationFrame(draw); // Skip if no context yet

    // --- Clear Canvas ---
    ctx.fillStyle = "#000"; // Black background
    ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

    // --- Draw Middle Line ---
    ctx.strokeStyle = "#555";
    ctx.lineWidth = 4;
    ctx.beginPath();
    ctx.setLineDash([10, 10]);
    ctx.moveTo(CANVAS_WIDTH / 2, 0);
    ctx.lineTo(CANVAS_WIDTH / 2, CANVAS_HEIGHT);
    ctx.stroke();
    ctx.setLineDash([]); // Reset line dash

    if (!liveGame) {
        // Show loading or waiting state if game data not loaded yet
        ctx.fillStyle = "#fff";
        ctx.font = "30px Arial";
        ctx.textAlign = "center";
        ctx.fillText("Loading Game...", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2);
        animationFrameId = requestAnimationFrame(draw); // Continue loop
        return;
    }

    // --- Draw Paddles ---
    ctx.fillStyle = "#fff"; // White paddles
    ctx.fillRect(0, paddle1Y, PADDLE_WIDTH, PADDLE_HEIGHT); // Player 1 (left)
    ctx.fillRect(CANVAS_WIDTH - PADDLE_WIDTH, paddle2Y, PADDLE_WIDTH, PADDLE_HEIGHT); // Player 2 (right)

    // --- Draw Ball ---
    ctx.beginPath();
    ctx.arc(ball.x, ball.y, BALL_RADIUS, 0, 2 * Math.PI);
    ctx.fillStyle = "#fff"; // White ball
    ctx.fill();
    ctx.closePath();

    // --- Draw Scores ---
    ctx.font = "40px 'Courier New', Courier, monospace";
    ctx.textAlign = "center";
    ctx.fillText(score.player1.toString(), CANVAS_WIDTH / 4, 60);
    ctx.fillText(score.player2.toString(), (3 * CANVAS_WIDTH) / 4, 60);

    // --- Game Over Display ---
    if (gameOver) {
        ctx.fillStyle = "rgba(0, 0, 0, 0.7)";
        ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT); // Dim background

        ctx.fillStyle = "#fff";
        ctx.font = "50px Arial";
        ctx.textAlign = "center";
        ctx.fillText("GAME OVER", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2 - 50);

        ctx.font = "30px Arial";
         if (winner) {
             const winnerName = encodeHashToBase64(winner) === encodeHashToBase64(liveGame.player_1) ? "Player 1" : "Player 2";
             ctx.fillText(`${winnerName} Wins!`, CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2);
         } else {
             ctx.fillText("Game Finished", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2); // Fallback
         }
         ctx.font = "40px Arial";
         ctx.fillText(`${score.player1} - ${score.player2}`, CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2 + 50);

        // Stop requesting new frames
        return;
    }

    // --- Update Logic ---
    // Only Player 1 updates ball physics and score
    if (isPlayer1) {
        updateBallAndScore(); // This now includes scoring and game over check
    }
    // Both players continue sending paddle updates (throttled) even if ball isn't moving
    // sendPaddleUpdate(); // Called on keydown now

    // --- Request Next Frame ---
    animationFrameId = requestAnimationFrame(draw);
  }

  onMount(async () => {
    client = await appClientContext.getClient();
    await fetchGameState(); // Fetch initial state
    if (canvas) {
        ctx = canvas.getContext("2d")!;
    }
    if (!gameOver) { // Only start loop if game isn't already finished
        draw(); // Start game loop
        window.addEventListener("keydown", handleKeyDown);
        unsubscribeFromSignals = subscribeToGameSignals();
    } else {
         draw(); // Draw final state if already over
         window.addEventListener("keydown", handleKeyDown); // Allow input for potential future menus
         unsubscribeFromSignals = subscribeToGameSignals(); // Still listen in case of late signals?
    }
  });

  onDestroy(() => {
    cancelAnimationFrame(animationFrameId);
    window.removeEventListener("keydown", handleKeyDown);
    if (unsubscribeFromSignals) unsubscribeFromSignals();
  });

 // Helper for deserialization if needed (replace with actual if using holochain-client helpers)
  class Game {
     static deserialize(data: any): Game {
         // Manual deserialization or use library helpers if available
         return data as Game; // Basic cast, potentially unsafe
     }
  }
  class Score { /* ... */ }
  class UpdateGameInput { /* ... */ }


</script>

<div class="game-container">
    <div class="game-window">
    <div class="players-info">
        <div class="player player1">
        P1: {#if liveGame}{truncatePubkey(liveGame.player_1)}{:else}Loading...{/if}
        </div>
        <div class="player player2">
        P2: {#if liveGame && liveGame.player_2}{truncatePubkey(liveGame.player_2)}{:else}Waiting...{/if}
        </div>
    </div>
    <canvas bind:this={canvas} width={CANVAS_WIDTH} height={CANVAS_HEIGHT}></canvas>
     {#if gameOver}
        <div class="game-over-menu">
             <button on:click={goBackToLobby}>Back to Lobby</button>
             </div>
     {/if}
    </div>
</div>

<style>
  .game-container {
      display: flex;
      justify-content: center;
      align-items: center;
      flex-direction: column;
  }
  .game-window {
    position: relative; /* Needed for absolute positioning of overlays */
    margin-top: 10px;
  }
  .players-info {
    position: absolute;
    top: 5px; /* Closer to top */
    left: 0; /* Align left */
    width: 100%;
    display: flex;
    justify-content: space-between; /* Space out P1 and P2 */
    padding: 0 15px; /* Padding left/right */
    color: orange;
    font-size: 0.9rem; /* Smaller font */
    font-weight: bold;
    z-index: 1; /* Above canvas */
    pointer-events: none; /* Allow clicks through to canvas */
  }
  .player {
     background-color: rgba(0,0,0,0.5); /* Slight background for readability */
     padding: 2px 5px;
     border-radius: 3px;
  }
  canvas {
    background-color: #000;
    display: block;
    margin: 0 auto;
    border: 2px solid #646cff;
  }
  .game-over-menu {
      position: absolute;
      bottom: 20px;
      left: 50%;
      transform: translateX(-50%);
      z-index: 10; /* Above canvas */
  }
   .game-over-menu button {
       font-size: 1.2rem;
       padding: 0.8rem 1.5rem;
       background-color: #646cff;
       color: white;
       border: none;
       border-radius: 5px;
       cursor: pointer;
   }
    .game-over-menu button:hover {
        background-color: #535bf2;
    }
</style>