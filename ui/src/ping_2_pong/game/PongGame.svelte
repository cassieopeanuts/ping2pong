<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { getContext } from "svelte";
  // Import types from holochain/client - ensure Action and Entry are imported
  import type { AppClient, ActionHash, AgentPubKey, Record, Entry, SignedActionHashed, Action } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  // Import local types and context
  import { clientContext, type ClientContext } from "../../contexts";
  import type { Game, Score, GameStatus, UpdateGameInput } from "../ping_2_pong/types";
  import { currentRoute } from "../../stores/routeStore";

  export let gameId: ActionHash;
  export let playerKey: AgentPubKey; // Passed as AgentPubKey object

  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // Game constants
  const CANVAS_WIDTH = 800;
  const CANVAS_HEIGHT = 600;
  const PADDLE_WIDTH = 10;
  const PADDLE_HEIGHT = 100;
  const BALL_RADIUS = 10;
  const WINNING_SCORE = 10; // Set winning score
  const PADDLE_SPEED = 15;
  const UPDATE_INTERVAL = 50; // ms interval for sending signal updates

  // Component State
  let gameRecord: Record | undefined; // Store the latest fetched full Record
  let liveGame: Game | undefined; // Store the deserialized Game data from the entry
  let isPlayer1 = false;
  let isPlayer2 = false;
  let paddle1Y = CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2;
  let paddle2Y = CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2;
  let ball = { x: CANVAS_WIDTH / 2, y: CANVAS_HEIGHT / 2, dx: 5, dy: 5 };
  let score = { player1: 0, player2: 0 };
  let gameOver = false;
  let winner: AgentPubKey | null = null;
  let errorMsg: string | null = null; // For displaying errors

  // Canvas and Animation
  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let animationFrameId: number;

  // Signal Handling
  let unsubscribeFromSignals: (() => void) | undefined;
  let lastPaddleUpdate = 0;
  let lastBallUpdate = 0;

  // --- Helper Functions ---

  function truncatePubkey(pubkey: AgentPubKey | null | undefined): string {
    if (!pubkey) return "N/A";
    try {
      const base64 = encodeHashToBase64(pubkey);
      return base64.slice(0, 8) + "..." + base64.slice(-6);
    } catch (e) {
        console.error("Error encoding pubkey:", e);
        return "Error";
    }
  }

  // --- Core Functions ---


  async function fetchGameState() {
    errorMsg = null;
    if (!client || !gameId) {
        errorMsg = "Client or Game ID missing. Cannot fetch game state.";
        console.error(errorMsg);
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

      if (result) {
        gameRecord = result;
        const recordEntry = result.entry;

        // FIX: Safely extract Entry using 'Present' key check ONLY
        let actualEntry: Entry | undefined = undefined;
        if (recordEntry && typeof recordEntry === 'object' && 'Present' in recordEntry && (recordEntry as any).Present) {
             // Check if the nested entry field exists (adjust path if needed based on client lib)
             const presentEntry = (recordEntry as { Present: Entry }).Present;
             // Check if presentEntry itself is valid (not null/undefined)
             if (presentEntry) {
                actualEntry = presentEntry;
             } else {
                 console.warn("RecordEntry has 'Present' key, but its value is null/undefined", recordEntry);
             }
        }
        // REMOVED the unsafe 'else if' fallback cast block

        if (actualEntry) {
            const game = actualEntry as unknown as Game;
            // ... rest of successful fetch logic as before ...
            if (!game || typeof game !== 'object' || !game.player_1 || !game.game_status) { /* ... */ return; }
            liveGame = game;
            console.log("Fetched game data:", liveGame);
            const myPubKeyB64 = encodeHashToBase64(playerKey);
            isPlayer1 = encodeHashToBase64(liveGame.player_1) === myPubKeyB64;
            isPlayer2 = !!liveGame.player_2 && (encodeHashToBase64(liveGame.player_2) === myPubKeyB64);
            console.log(`Is Player 1: ${isPlayer1}, Is Player 2: ${isPlayer2}`);
            if (score.player1 === 0 && score.player2 === 0) {
                paddle1Y = liveGame.player_1_paddle ?? (CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2);
                paddle2Y = liveGame.player_2_paddle ?? (CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2);
                ball.x = liveGame.ball_x ?? (CANVAS_WIDTH / 2); ball.y = liveGame.ball_y ?? (CANVAS_HEIGHT / 2);
                ball.dx = 5 * (Math.random() > 0.5 ? 1 : -1); ball.dy = 5 * (Math.random() > 0.5 ? 1 : -1);
            }
            if (liveGame.game_status?.type === 'Finished') { console.log("Game already finished on load."); gameOver = true; }

        } else {
            // This block now correctly catches cases where the entry wasn't Present
            errorMsg = `Workspaceed record for game ${encodeHashToBase64(gameId)} does not contain a present entry.`;
            console.error(errorMsg, result);
            liveGame = undefined; gameRecord = undefined;
        }

      } else {
          errorMsg = `Failed to fetch record for gameId: ${encodeHashToBase64(gameId)}`;
          console.error(errorMsg);
          liveGame = undefined; gameRecord = undefined;
      }
    } catch (e) {
      errorMsg = `Error fetching game state: ${(e as Error).message}`;
      console.error(errorMsg, e);
      liveGame = undefined; gameRecord = undefined;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (gameOver || !liveGame) return;

    let moved = false;
    if (isPlayer1) {
      if (e.key === "ArrowUp" || e.key === "w" || e.key === "W") {
        paddle1Y = Math.max(0, paddle1Y - PADDLE_SPEED); moved = true;
      } else if (e.key === "ArrowDown" || e.key === "s" || e.key === "S") {
        paddle1Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle1Y + PADDLE_SPEED); moved = true;
      }
    } else if (isPlayer2) {
      if (e.key === "ArrowUp" || e.key === "w" || e.key === "W") {
        paddle2Y = Math.max(0, paddle2Y - PADDLE_SPEED); moved = true;
      } else if (e.key === "ArrowDown" || e.key === "s" || e.key === "S") {
        paddle2Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle2Y + PADDLE_SPEED); moved = true;
      }
    }
    if (moved) sendPaddleUpdate();
  }

  async function sendPaddleUpdate() {
    if (gameOver || !client || !liveGame) return;
    const now = Date.now();
    if (now - lastPaddleUpdate < UPDATE_INTERVAL) return;
    lastPaddleUpdate = now;

    const signal = { type: "PaddleUpdate", game_id: gameId, player: playerKey, paddle_y: isPlayer1 ? paddle1Y : paddle2Y };
    try {
      await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "send_signal", payload: signal });
    } catch (e) { console.error("Error sending paddle update signal:", e); }
  }

  async function sendBallUpdate() {
    if (gameOver || !isPlayer1 || !client || !liveGame) return;
    const now = Date.now();
    if (now - lastBallUpdate < UPDATE_INTERVAL) return;
    lastBallUpdate = now;

    const signal = { type: "BallUpdate", game_id: gameId, ball_x: ball.x, ball_y: ball.y, ball_dx: ball.dx, ball_dy: ball.dy };
    try {
       await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "send_signal", payload: signal });
    } catch (e) { console.error("Error sending ball update signal:", e); }
  }

  function subscribeToGameSignals() {
    if (!client) return undefined;
    return client.on("signal", (signalPayload: any) => {
      if (gameOver || !liveGame) return;
      if (!signalPayload || !signalPayload.type || !signalPayload.game_id) return;
      // Check if signal is for this game (compare base64 representations for safety)
      if (encodeHashToBase64(signalPayload.game_id) !== encodeHashToBase64(gameId)) return;

      const myPubKeyB64 = encodeHashToBase64(playerKey);

      try {
          if (signalPayload.type === "PaddleUpdate") {
            // Ensure player field exists and is AgentPubKey before encoding
            if (signalPayload.player && typeof signalPayload.player === 'object' && signalPayload.player.length === 39 && encodeHashToBase64(signalPayload.player) !== myPubKeyB64) {
              if (isPlayer1) paddle2Y = signalPayload.paddle_y;
              else if (isPlayer2) paddle1Y = signalPayload.paddle_y;
            }
          } else if (signalPayload.type === "BallUpdate") {
            if (!isPlayer1) { // P2 updates ball based on P1's signal
              ball.x = signalPayload.ball_x; ball.y = signalPayload.ball_y;
              ball.dx = signalPayload.ball_dx; ball.dy = signalPayload.ball_dy;
            }
          } else if (signalPayload.type === "GameOver") {
            console.log("Received GameOver signal:", signalPayload);
            // Ensure winner field exists and is AgentPubKey or null
            const remoteWinner = (signalPayload.winner && typeof signalPayload.winner === 'object' && signalPayload.winner.length === 39) ? signalPayload.winner as AgentPubKey : null;
            if (!gameOver) handleRemoteGameOver(remoteWinner);
          }
      } catch (e) { console.error("Error processing signal:", signalPayload, e); }
    });
  }

  // Only Player 1 runs the main physics and scoring
  function updateBallAndScore() {
    if (gameOver || !isPlayer1 || !liveGame) return;

    ball.x += ball.dx;
    ball.y += ball.dy;

    // Wall collisions
    if (ball.y + BALL_RADIUS > CANVAS_HEIGHT || ball.y - BALL_RADIUS < 0) {
      ball.dy = -ball.dy;
      ball.y = Math.max(BALL_RADIUS, Math.min(CANVAS_HEIGHT - BALL_RADIUS, ball.y));
    }

    // Paddle collisions
    let hitPaddle = false;
    // P1 Paddle
    if (ball.dx < 0 && ball.x - BALL_RADIUS < PADDLE_WIDTH && ball.x > BALL_RADIUS && ball.y > paddle1Y && ball.y < paddle1Y + PADDLE_HEIGHT) {
        ball.dx = -ball.dx * 1.05; ball.x = PADDLE_WIDTH + BALL_RADIUS;
        ball.dy = (ball.y - (paddle1Y + PADDLE_HEIGHT / 2)) * 0.35; hitPaddle = true;
    }
    // P2 Paddle
    else if (ball.dx > 0 && ball.x + BALL_RADIUS > CANVAS_WIDTH - PADDLE_WIDTH && ball.x < CANVAS_WIDTH - BALL_RADIUS && ball.y > paddle2Y && ball.y < paddle2Y + PADDLE_HEIGHT) {
        ball.dx = -ball.dx * 1.05; ball.x = CANVAS_WIDTH - PADDLE_WIDTH - BALL_RADIUS;
        ball.dy = (ball.y - (paddle2Y + PADDLE_HEIGHT / 2)) * 0.35; hitPaddle = true;
    }

    // Scoring
    let scored = false;
    if (ball.x + BALL_RADIUS < 0) { score.player2++; scored = true; }
    else if (ball.x - BALL_RADIUS > CANVAS_WIDTH) { score.player1++; scored = true; }

    // Post-score logic
    if (scored) {
      console.log(`Score: ${score.player1} - ${score.player2}`);
      if (score.player1 >= WINNING_SCORE || score.player2 >= WINNING_SCORE) {
        // Determine winner based on score; ensure liveGame exists
        winner = score.player1 >= WINNING_SCORE ? liveGame.player_1 : liveGame.player_2;
        gameOver = true;
        if(winner) console.log("Game Over! Winner:", truncatePubkey(winner));
        handleLocalGameOver(); // Trigger backend updates etc.
      } else {
        // Reset ball if no winner yet
        ball.x = CANVAS_WIDTH / 2; ball.y = CANVAS_HEIGHT / 2;
        ball.dx = 5 * (score.player1 > score.player2 ? -1 : 1); // Serve towards loser
        ball.dy = 5 * (Math.random() > 0.5 ? 1 : -1);
        lastBallUpdate = 0; sendBallUpdate(); // Force sync after reset
      }
    } else if (hitPaddle) {
      lastBallUpdate = 0; sendBallUpdate(); // Force sync after hit
    } else {
      sendBallUpdate(); // Regular update
    }
  }

  // Called when game ends locally (P1 detects win)
  async function handleLocalGameOver() {
      // Ensure essential records/state exist before proceeding
      if (!liveGame || !gameRecord || !gameRecord.signed_action) {
          console.error("Cannot handle game over: Missing liveGame, gameRecord, or signed_action");
          errorMsg = "Error handling game over: Missing essential game data.";
          return;
      }
      console.log("Handling local game over...");

      // Safely extract the entry data
      let actualEntry: Entry | undefined = undefined;
      const recordEntry = gameRecord.entry;
      // Check for the structure { Present: Entry }
      if (recordEntry && typeof recordEntry === 'object' && 'Present' in recordEntry && (recordEntry as any).Present) {
          actualEntry = (recordEntry as { Present: Entry }).Present;
      }
      // Add other checks here if needed based on client library behavior

      if (!actualEntry) {
          errorMsg = "Could not extract latest game state from record entry.";
          console.error(errorMsg, gameRecord.entry);
          return;
      }

      // Cast the extracted Entry data to our Game interface
      const latestGameState = actualEntry as unknown as Game;

      // Double-check required fields after casting
       if (!latestGameState || !latestGameState.player_1 || !latestGameState.game_status) {
           errorMsg = "Extracted game state is invalid."; console.error(errorMsg, latestGameState); return;
       }

      // Determine original game hash correctly before try blocks
      let determined_original_hash: ActionHash;
      const current_action_hash = gameRecord.signed_action.hashed.hash;
      // Access action content via signed_action.hashed.content
      const actionContent: Action = gameRecord.signed_action.hashed.content;

      // Check action structure for Update variant
      if (actionContent && typeof actionContent === 'object' && 'Update' in actionContent && (actionContent as any).Update?.original_action_address) {
           // Explicitly type cast for clarity when accessing nested field
           determined_original_hash = (actionContent as { Update: { original_action_address: ActionHash } }).Update.original_action_address;
           console.log("Determined original hash from Update action:", encodeHashToBase64(determined_original_hash));
      } else { // Assume Create or other non-update action
           determined_original_hash = current_action_hash;
           console.log("Determined original hash (using current action hash):", encodeHashToBase64(determined_original_hash));
      }

      // --- Proceed with backend calls ---

      // 1. Update Game Status to Finished
      try {
            const finishedGameState: Game = {
                // Ensure all fields from Game interface are included
                player_1: latestGameState.player_1,
                player_2: latestGameState.player_2,
                created_at: latestGameState.created_at, // Keep original creation time
                game_status: { type: 'Finished' }, // Set status
                player_1_paddle: paddle1Y, // Snapshot final positions
                player_2_paddle: paddle2Y,
                ball_x: ball.x,
                ball_y: ball.y,
            };

            // Use the previously determined hashes
            const updatePayload: UpdateGameInput = {
                 original_game_hash: determined_original_hash,
                 previous_game_hash: current_action_hash, // Hash of the record we fetched
                 updated_game: finishedGameState,
            };

            console.log("Updating game status to Finished with payload:", updatePayload);
            await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "update_game", payload: updatePayload });
            console.log("Game status updated.");
       } catch (e) { console.error("Error updating game status:", e); errorMsg="Failed to update game status."; /* Continue to score saving */ }

       // 2. Save Scores
       try {
           if (!liveGame || !liveGame.player_1) { throw new Error("liveGame or player_1 missing"); }

           const score1Payload: Score = {
               game_id: determined_original_hash, // Use determined original hash
               player: liveGame.player_1,
               player_points: score.player1,
               created_at: Date.now() // JS timestamp (number)
           };
           await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "create_score", payload: score1Payload });

           if (liveGame.player_2) {
                const score2Payload: Score = {
                   game_id: determined_original_hash, // Use determined original hash
                   player: liveGame.player_2,
                   player_points: score.player2,
                   created_at: Date.now() // JS timestamp (number)
                };
                await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "create_score", payload: score2Payload });
                console.log("Scores saved.");
           } else { console.log("Player 2 score not saved as Player 2 does not exist."); }
       } catch (e) { console.error("Error saving scores:", e); errorMsg = "Failed to save scores."; }

       // 3. (Optional) Send GameOver signal
       try {
           const gameOverSignal = { type: "GameOver", game_id: gameId, winner: winner, score1: score.player1, score2: score.player2 };
           await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "send_signal", payload: gameOverSignal });
           console.log("GameOver signal sent.");
       } catch(e) { console.error("Error sending GameOver signal:", e); }

       // 4. (Future) Save Statistics
       // await saveStatistics();
  }

  // Handle game over triggered by remote signal
  function handleRemoteGameOver(remoteWinner: AgentPubKey | null) {
      if (gameOver) return;
      console.log("Handling remote game over signal...");
      gameOver = true;
      winner = remoteWinner;
      // UI updates automatically based on gameOver flag in draw()
  }

  // Navigate back to dashboard
  function goBackToLobby() {
      currentRoute.set("dashboard");
      // Optional: Reset component state if needed when navigating away
      // gameOver = false; winner = null; score = {player1: 0, player2: 0};
      // liveGame = undefined; gameRecord = undefined;
  }

  // Main canvas drawing loop
  function draw() {
    if (!ctx) { animationFrameId = requestAnimationFrame(draw); return; }

    // Clear, draw background, midline
    ctx.fillStyle = "#000"; ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
    ctx.strokeStyle = "#555"; ctx.lineWidth = 4; ctx.beginPath();
    ctx.setLineDash([10, 10]); ctx.moveTo(CANVAS_WIDTH / 2, 0); ctx.lineTo(CANVAS_WIDTH / 2, CANVAS_HEIGHT);
    ctx.stroke(); ctx.setLineDash([]);

    // Display Loading / Error
    if (!liveGame) {
        ctx.fillStyle = "#fff"; ctx.font = "30px Arial"; ctx.textAlign = "center";
        ctx.fillText(errorMsg || "Loading Game...", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2);
        // Keep requesting frames until loaded or confirmed error state prevents loading
        if (!errorMsg && !gameOver) animationFrameId = requestAnimationFrame(draw);
        return;
    }

    // Draw Game Elements
    ctx.fillStyle = "#fff";
    ctx.fillRect(0, paddle1Y, PADDLE_WIDTH, PADDLE_HEIGHT); // P1
    ctx.fillRect(CANVAS_WIDTH - PADDLE_WIDTH, paddle2Y, PADDLE_WIDTH, PADDLE_HEIGHT); // P2
    ctx.beginPath(); ctx.arc(ball.x, ball.y, BALL_RADIUS, 0, 2 * Math.PI); ctx.fill(); ctx.closePath(); // Ball

    // Draw Scores
    ctx.font = "40px 'Courier New', Courier, monospace"; ctx.textAlign = "center";
    ctx.fillText(score.player1.toString(), CANVAS_WIDTH / 4, 60);
    ctx.fillText(score.player2.toString(), (3 * CANVAS_WIDTH) / 4, 60);

    // Game Over Display
    if (gameOver) {
        ctx.fillStyle = "rgba(0, 0, 0, 0.7)"; ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
        ctx.fillStyle = "#fff"; ctx.font = "50px Arial"; ctx.textAlign = "center";
        ctx.fillText("GAME OVER", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2 - 50);
        ctx.font = "30px Arial";
         if (winner && liveGame) {
             const winnerName = encodeHashToBase64(winner) === encodeHashToBase64(liveGame.player_1) ? "Player 1" : "Player 2";
             ctx.fillText(`${winnerName} Wins!`, CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2);
         } else { ctx.fillText("Game Finished", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2); }
         ctx.font = "40px Arial";
         ctx.fillText(`${score.player1} - ${score.player2}`, CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2 + 50);
        // Stop the animation loop once Game Over is drawn
        return;
    }

    // Update Logic (P1 only) & Request Next Frame if game not over
    if (isPlayer1) updateBallAndScore();
    animationFrameId = requestAnimationFrame(draw);
  }

  // Component Lifecycle
  onMount(async () => {
    client = await appClientContext.getClient();
    await fetchGameState(); // Fetch state first
    if (canvas) {
        ctx = canvas.getContext("2d")!; // Get context if canvas exists
    } else {
        console.error("Canvas element not found on mount.");
        errorMsg = "Failed to initialize canvas.";
    }

    // Start game loop only if successfully loaded and not already over
    if (liveGame && !gameOver && ctx) {
        draw();
        window.addEventListener("keydown", handleKeyDown);
        unsubscribeFromSignals = subscribeToGameSignals();
    } else if (liveGame && gameOver && ctx) {
         draw(); // Draw final state if loaded but already over
    } else {
        // Handle case where fetch failed or canvas failed
        if (ctx) draw(); // Attempt to draw error message if context exists
        else console.error("Cannot draw initial state: No context and/or no game data.");
    }
  });

  onDestroy(() => {
    cancelAnimationFrame(animationFrameId);
    window.removeEventListener("keydown", handleKeyDown);
    if (unsubscribeFromSignals) unsubscribeFromSignals();
  });

</script>

<div class="game-container">
    {#if errorMsg && !ctx} <p class="error-message">Error: {errorMsg}</p>
    {/if}
    <div class="game-window">
        <div class="players-info">
            <div class="player player1">P1: {#if liveGame?.player_1}{truncatePubkey(liveGame.player_1)}{:else}Loading...{/if}</div>
            <div class="player player2">P2: {#if liveGame?.player_2}{truncatePubkey(liveGame.player_2)}{:else}Waiting...{/if}</div>
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
  .game-container { display: flex; justify-content: center; align-items: center; flex-direction: column; }
  .error-message { color: red; margin-bottom: 10px; }
  .game-window { position: relative; margin-top: 10px; }
  .players-info { position: absolute; top: 5px; left: 0; width: 100%; display: flex; justify-content: space-between; padding: 0 15px; color: orange; font-size: 0.9rem; font-weight: bold; z-index: 1; pointer-events: none; }
  .player { background-color: rgba(0,0,0,0.5); padding: 2px 5px; border-radius: 3px; }
  canvas { background-color: #000; display: block; margin: 0 auto; border: 2px solid #646cff; }
  .game-over-menu { position: absolute; bottom: 20px; left: 50%; transform: translateX(-50%); z-index: 10; }
  .game-over-menu button { font-size: 1.2rem; padding: 0.8rem 1.5rem; background-color: #646cff; color: white; border: none; border-radius: 5px; cursor: pointer; }
  .game-over-menu button:hover { background-color: #535bf2; }
</style>