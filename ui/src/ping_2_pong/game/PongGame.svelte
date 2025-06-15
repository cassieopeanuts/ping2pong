<script lang="ts">
  import { onMount, onDestroy, getContext, createEventDispatcher } from "svelte";
  import type { AppClient, ActionHash, AgentPubKey, Record, Entry } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  import { clientContext, type ClientContext } from "../../contexts";
  import { decode } from "@msgpack/msgpack";
  import type { Game, Score, GameStatus, UpdateGameInput, PaddleUpdateSignal, BallUpdateSignal, GameOverSignal, ScoreUpdateSignal } from "../ping_2_pong/types";
  import { getOrFetchProfile, type DisplayProfile } from "../../stores/profilesStore";
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "../../holochainConfig";

  // Create dispatcher to send events up to the parent (App.svelte)
  const dispatch = createEventDispatcher();

  // Component Props passed from App.svelte
  export let gameId: ActionHash; // The ORIGINAL ActionHash of the game
  export let playerKey: AgentPubKey; // The current user's public key

  // Holochain Client
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // Game Constants
  const CANVAS_WIDTH = 800;
  const CANVAS_HEIGHT = 600;
  const PADDLE_WIDTH = 10;
  const PADDLE_HEIGHT = 100;
  const BALL_RADIUS = 10;
  const WINNING_SCORE = 10;
  const PADDLE_SPEED = 25;
  const UPDATE_INTERVAL = 50; // ms interval for sending signal updates

  // Component State
  let gameRecord: Record | undefined; // Stores the latest fetched Holochain record for the game
  let liveGame: Game | undefined; // Stores the deserialized Game data from the entry (set only when ready)
  let isPlayer1 = false; // Flag indicating if the current user is Player 1
  let isPlayer2 = false; // Flag indicating if the current user is Player 2
  let paddle1Y = CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2; // Player 1 paddle Y position
  let paddle2Y = CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2; // Player 2 paddle Y position
  let ball = { x: CANVAS_WIDTH / 2, y: CANVAS_HEIGHT / 2, dx: 5, dy: 5 }; // Ball position and velocity
  let score = { player1: 0, player2: 0 }; // Current scores
  let gameOver = false; // Flag indicating if the game has ended
  let winner: AgentPubKey | null = null; // Stores the winner's public key if game is over
  let errorMsg: string | null = null; // Stores any error message for display
  let loadingMsg: string | null = "Initializing game..."; // Loading message

  // Player Profiles
  let player1Profile: DisplayProfile | null = null;
  let player2Profile: DisplayProfile | null = null;

  // Canvas & Animation
  let canvas: HTMLCanvasElement; // Reference to the canvas element
  let ctx: CanvasRenderingContext2D; // Canvas 2D rendering context
  let animationFrameId: number; // ID for the requestAnimationFrame loop

  // Signal Handling
  let unsubscribeFromSignals: (() => void) | undefined; // Function to unsubscribe from signal listener
  let lastPaddleUpdate = 0; // Timestamp of the last paddle update sent
  let lastBallUpdate = 0; // Timestamp of the last ball update sent

  // Retry mechanism state
  let retryTimeoutId: ReturnType<typeof setTimeout> | undefined;
  let retryCount = 0;
  const MAX_RETRIES = 5; // e.g., try 5 times
  const RETRY_DELAY = 1000; // 1 second delay

  // --- Helper Functions ---

  // Shortens a public key for display purposes
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

  // Fetches the latest game state, returns the Game object or null if not ready/error
  async function fetchGameState(): Promise<Game | null> {
    // Don't clear errorMsg here, initializeGame handles status display
    if (!client || !gameId) {
        console.error("[PongGame fetchGameState] Client or Game ID missing.");
        errorMsg = "Client/Game ID missing"; // Set error for display
        return null;
    }
    try {
      console.log(`[PongGame fetchGameState] Attempting fetch for game: ${encodeHashToBase64(gameId)}`);
      // Call the zome function to get the latest game record based on the original hash
      const result: Record | null = await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong",
        zome_name: "ping_2_pong",
        fn_name: "get_latest_game", // Gets the record associated with the latest update action
        payload: gameId, // Pass the original game hash
      });

      if (result) {
        gameRecord = result; // Store latest record
        const recordEntry = result.entry;
        let actualEntry: Entry | undefined = undefined;
        // Safely extract the Entry object from the Record
        if (recordEntry && typeof recordEntry === 'object' && 'Present' in recordEntry && (recordEntry as any).Present) {
             const presentEntry = (recordEntry as { Present: Entry }).Present;
             if (presentEntry) actualEntry = presentEntry;
        }

        // Ensure we have a valid App entry containing Uint8Array data
        if (actualEntry && actualEntry.entry_type === 'App' && actualEntry.entry instanceof Uint8Array) {
            try {
                // Decode the MessagePack bytes into a Game object
                const decodedGame = decode(actualEntry.entry) as Game;
                console.log("[PongGame fetchGameState] Decoded game state:", decodedGame);
                // *** Check if game is ready (InProgress and Player 2 exists) ***
                if (decodedGame.game_status === 'InProgress' && decodedGame.player_2) {
                    console.log("[PongGame fetchGameState] Game state is InProgress with Player 2. Ready.");
                    return decodedGame; // Return the ready game state
                } else {
                    console.log(`[PongGame fetchGameState] Game state not ready yet (Status: ${decodedGame.game_status}, P2: ${decodedGame.player_2 ? 'Set' : 'Null'}). Will retry.`);
                    return null; // Indicate not ready
                }
            } catch (decodeError) {
                // Handle errors during MessagePack decoding
                console.error("[PongGame fetchGameState] Failed to decode entry:", decodeError);
                errorMsg = "Failed to decode game data";
                return null; // Error decoding
            }
        } else {
            // Handle cases where the entry data is missing or not in the expected format
            console.error("[PongGame fetchGameState] Could not extract valid App entry.", result);
            errorMsg = "Invalid game record structure";
            return null; // Invalid entry structure
        }
      } else {
          // Handle case where the game record itself wasn't found
          console.warn(`[PongGame fetchGameState] Failed to fetch record for gameId: ${encodeHashToBase64(gameId)}. Maybe DHT delay?`);
          // Don't set errorMsg yet, retry might succeed
          return null; // Record not found (could be DHT delay)
      }
    } catch (e) {
      // Handle errors during the zome call
      console.error("[PongGame fetchGameState] Error fetching game state:", e);
      errorMsg = `Error fetching game: ${(e as Error).message}`;
      return null; // Zome call error
    }
  }

  // Initializes the game, retrying fetchGameState if needed
  async function initializeGame() {
      console.log(`[PongGame initializeGame] Starting initialization. Retry count: ${retryCount}`);
      loadingMsg = `Initializing game... (Attempt ${retryCount + 1})`;
      errorMsg = null; // Clear previous errors

      const fetchedGame = await fetchGameState();

      if (fetchedGame) {
          // --- Game Ready ---
          loadingMsg = null; // Clear loading message
          liveGame = fetchedGame; // Set the live game state

          // Identify players based on the confirmed state
          const myPubKeyB64 = encodeHashToBase64(playerKey);
          isPlayer1 = encodeHashToBase64(liveGame.player_1) === myPubKeyB64;
          // We know player_2 exists because we checked for it in fetchGameState
          isPlayer2 = encodeHashToBase64(liveGame.player_2!) === myPubKeyB64;
          console.log(`[PongGame initializeGame] Player role identified: isPlayer1=${isPlayer1}, isPlayer2=${isPlayer2}`);

          // Fetch profiles
          if (liveGame.player_1) {
            getOrFetchProfile(client, liveGame.player_1).then(profile => player1Profile = profile);
          }
          if (liveGame.player_2) {
            getOrFetchProfile(client, liveGame.player_2).then(profile => player2Profile = profile);
          }

          // Initialize positions (only if score is 0)
          if (score.player1 === 0 && score.player2 === 0) {
              paddle1Y = liveGame.player_1_paddle ?? (CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2);
              paddle2Y = liveGame.player_2_paddle ?? (CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2);
              ball.x = liveGame.ball_x ?? (CANVAS_WIDTH / 2);
              ball.y = liveGame.ball_y ?? (CANVAS_HEIGHT / 2);
              ball.dx = 5 * (Math.random() > 0.5 ? 1 : -1);
              ball.dy = 5 * (Math.random() > 0.5 ? 1 : -1);
              console.log("[PongGame initializeGame] Initialized positions.");
          }

          // Start the game loop and listeners
          startGameLoop();

      } else if (retryCount < MAX_RETRIES) {
          // --- Game Not Ready, Retry ---
          retryCount++;
          console.log(`[PongGame initializeGame] Game not ready, scheduling retry #${retryCount} in ${RETRY_DELAY}ms`);
          retryTimeoutId = setTimeout(initializeGame, RETRY_DELAY); // Schedule next attempt
      } else {
          // --- Max Retries Reached ---
          console.error(`[PongGame initializeGame] Failed to fetch ready game state after ${MAX_RETRIES + 1} attempts.`);
          loadingMsg = null; // Clear loading message
          errorMsg = "Failed to load game state after multiple attempts. Please exit and try again.";
          // Keep drawing to show the error message
          if (ctx) draw();
      }
  }

  // Starts the main game loop and sets up listeners
  function startGameLoop() {
      if (!ctx) {
          console.error("[PongGame startGameLoop] Canvas context not available!");
          errorMsg = "Canvas failed to initialize.";
          return;
      }
      if (animationFrameId) {
          console.warn("[PongGame startGameLoop] Game loop already running?");
          return; // Avoid starting multiple loops
      }
      console.log("[PongGame startGameLoop] Starting game loop and listeners.");
      gameOver = false; // Ensure game isn't marked over
      draw(); // Start drawing loop
      window.addEventListener("keydown", handleKeyDown); // Listen for keyboard input
      unsubscribeFromSignals = subscribeToGameSignals(); // Subscribe to game signals
  }


  // Handles keyboard input ('ArrowUp', 'ArrowDown', 'w', 's') for paddle movement
  function handleKeyDown(e: KeyboardEvent) {
    if (gameOver || !liveGame) return; // Ignore input if game is over or not loaded

    let moved = false; // Flag to track if the paddle actually moved
    // Player 1 controls
    if (isPlayer1) {
      if (e.key === "ArrowUp" || e.key === "w" || e.key === "W") {
        paddle1Y = Math.max(0, paddle1Y - PADDLE_SPEED); // Move up, clamp at top
        moved = true;
      } else if (e.key === "ArrowDown" || e.key === "s" || e.key === "S") {
        paddle1Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle1Y + PADDLE_SPEED); // Move down, clamp at bottom
        moved = true;
      }
    // Player 2 controls
    } else if (isPlayer2) {
      if (e.key === "ArrowUp" || e.key === "w" || e.key === "W") {
        paddle2Y = Math.max(0, paddle2Y - PADDLE_SPEED); // Move up, clamp at top
        moved = true;
      } else if (e.key === "ArrowDown" || e.key === "s" || e.key === "S") {
        paddle2Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle2Y + PADDLE_SPEED); // Move down, clamp at bottom
        moved = true;
      }
    }
    // If the paddle moved, send an update signal
    if (moved) sendPaddleUpdate();
  }

  // Sends the current player's paddle position update signal to the backend
  async function sendPaddleUpdate() {
    // Throttle updates to prevent sending too many signals
    const now = Date.now();
    if (gameOver || !client || !liveGame || !gameId || (now - lastPaddleUpdate < UPDATE_INTERVAL)) return;
    lastPaddleUpdate = now; // Update timestamp of last sent signal

    // Prepare payload matching the backend's PaddleUpdatePayload struct
    const payload = {
        game_id: gameId, // The original ActionHash identifying the game
        paddle_y: Math.round(isPlayer1 ? paddle1Y : paddle2Y) // Send the current Y position, rounded
    };

    try {
      // Call the specific backend function to send the signal
      await client.callZome({
          cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
          fn_name: "send_paddle_update", // Use the dedicated function
          payload: payload
      });
    } catch (e) { console.error("Error sending paddle update signal:", e); }
  }

  // Sends the current ball position and velocity update signal (only Player 1 does this)
  async function sendBallUpdate() {
    // Throttle updates and ensure only Player 1 sends these signals
    const now = Date.now();
    if (gameOver || !isPlayer1 || !client || !liveGame || !gameId || (now - lastBallUpdate < UPDATE_INTERVAL)) return;
    lastBallUpdate = now; // Update timestamp

    // Prepare payload matching the backend's BallUpdatePayload struct
    const payload = {
        game_id: gameId, // The original ActionHash identifying the game
        ball_x: Math.round(ball.x),
        ball_y: Math.round(ball.y),
        ball_dx: Math.round(ball.dx), 
        ball_dy: Math.round(ball.dy)  
    };

    try {
      // Call the specific backend function
      await client.callZome({
          cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
          fn_name: "send_ball_update", // Use the dedicated function
          payload: payload
      });
    } catch (e) { console.error("Error sending ball update signal:", e); }
  }

  async function sendScoreUpdate() {
    if (!client || !liveGame) return;
    try {
      await client.callZome({
        cap_secret: null,
        role_name : "ping_2_pong",
        zome_name : "ping_2_pong",
        fn_name   : "send_score_update",            
        payload: {
          game_id: gameId,
          score1 : score.player1,
          score2 : score.player2
        }
      });
    } catch (e) { console.error("Score update failed:", e); }
  }

  // Sets up the listener for incoming signals related to this specific game
  function subscribeToGameSignals() {
    if (!client) return;

    return client.on("signal", (raw: any) => {
      const s = raw?.App?.payload;
      if (!s || !s.type || gameOver) return;
      if (encodeHashToBase64(s.game_id) !== encodeHashToBase64(gameId)) return;

      const meB64 = encodeHashToBase64(playerKey);

      try {
        switch (s.type) {
          case "PaddleUpdate":
            if (encodeHashToBase64(s.player) !== meB64) {
              if (isPlayer1) paddle2Y = s.paddle_y;
              else           paddle1Y = s.paddle_y;
            }
            break;

          case "BallUpdate":
            if (!isPlayer1) {
              ball.x = s.ball_x; ball.y = s.ball_y;
              ball.dx = s.ball_dx; ball.dy = s.ball_dy;
            }
            break;

          case "ScoreUpdate":                            
            score.player1 = s.score1;
            score.player2 = s.score2;
            break;

          case "GameOver":
            handleRemoteGameOver(
              s.winner ?? null as AgentPubKey|null
            );
            break;
        }
      } catch(e) { console.error("signal parse err", e); }
    });
  }

  // Updates ball physics, checks for collisions and scoring (only Player 1 executes this)
  function updateBallAndScore() {
    if (gameOver || !isPlayer1 || !liveGame) return; // Guard: Only P1 runs physics

    // Move ball
    ball.x += ball.dx;
    ball.y += ball.dy;

    // Check for collisions with top/bottom walls
    if (ball.y + BALL_RADIUS > CANVAS_HEIGHT || ball.y - BALL_RADIUS < 0) {
      ball.dy = -ball.dy; // Reverse vertical velocity
      ball.y = Math.max(BALL_RADIUS, Math.min(CANVAS_HEIGHT - BALL_RADIUS, ball.y)); // Clamp position
    }

    // Check for collisions with paddles
    let hitPaddle = false;
    // Player 1 paddle collision logic
    if (ball.dx < 0 && ball.x - BALL_RADIUS < PADDLE_WIDTH && ball.x > BALL_RADIUS && ball.y > paddle1Y && ball.y < paddle1Y + PADDLE_HEIGHT) {
        ball.dx = -ball.dx * 1.05; // Reverse horizontal velocity, increase speed
        ball.x = PADDLE_WIDTH + BALL_RADIUS; // Reposition ball
        ball.dy = (ball.y - (paddle1Y + PADDLE_HEIGHT / 2)) * 0.35; // Add vertical angle
        hitPaddle = true;
    }
    // Player 2 paddle collision logic
    else if (ball.dx > 0 && ball.x + BALL_RADIUS > CANVAS_WIDTH - PADDLE_WIDTH && ball.x < CANVAS_WIDTH - BALL_RADIUS && ball.y > paddle2Y && ball.y < paddle2Y + PADDLE_HEIGHT) {
        ball.dx = -ball.dx * 1.05; // Reverse horizontal velocity, increase speed
        ball.x = CANVAS_WIDTH - PADDLE_WIDTH - BALL_RADIUS; // Reposition ball
        ball.dy = (ball.y - (paddle2Y + PADDLE_HEIGHT / 2)) * 0.35; // Add vertical angle
        hitPaddle = true;
    }

    // Check if a player scored (ball went past a paddle)
    let scored = false;
    if (ball.x + BALL_RADIUS < 0) {          // P2 scores
      score.player2++; scored = true; sendScoreUpdate();
    } else if (ball.x - BALL_RADIUS > CANVAS_WIDTH) { // P1 scores
      score.player1++; scored = true; sendScoreUpdate();
    }

    // Handle the outcome of the physics update
    if (scored) {
      console.log(`Score: ${score.player1} - ${score.player2}`);
      // Check if the game has been won
      if (score.player1 >= WINNING_SCORE || score.player2 >= WINNING_SCORE) {
        winner = score.player1 >= WINNING_SCORE ? liveGame.player_1 : liveGame.player_2; // Determine winner
        gameOver = true; // Set game over flag
        if(winner) console.log("Game Over! Winner:", truncatePubkey(winner));
        handleLocalGameOver(); // Trigger backend updates and game over signal
      } else {
        // If game not over, reset ball for the next point
        ball.x = CANVAS_WIDTH / 2;
        ball.y = CANVAS_HEIGHT / 2;
        ball.dx = 5 * (score.player1 > score.player2 ? -1 : 1); // Serve towards the player who lost the point
        ball.dy = 5 * (Math.random() > 0.5 ? 1 : -1); // Random vertical serve direction
        lastBallUpdate = 0; // Reset throttle timer for immediate update
        sendBallUpdate(); // Send the reset ball state
      }
    } else if (hitPaddle) {
      // If a paddle was hit, force a state update
      lastBallUpdate = 0; // Reset throttle timer
      sendBallUpdate();
    } else {
      // Send regular ball update if no score/hit
      sendBallUpdate();
    }
  }

  // Handles actions needed when the game ends locally (P1 detects win condition)
  async function handleLocalGameOver() {
      // Ensure necessary data is available
      if (!liveGame || !gameRecord || !gameRecord.signed_action) {
          console.error("Cannot handle game over: Missing liveGame, gameRecord, or signed_action");
          errorMsg = "Error handling game over: Missing essential game data.";
          return;
      }
      console.log("Handling local game over...");

      // Extract the latest game state from the fetched record's entry data
      let latestGameState: Game | undefined;
      const recordEntry = gameRecord.entry;
      if (recordEntry && typeof recordEntry === 'object' && 'Present' in recordEntry && (recordEntry as any).Present) {
          const presentEntry = (recordEntry as { Present: Entry }).Present;
          if (presentEntry && presentEntry.entry_type === 'App' && presentEntry.entry instanceof Uint8Array) {
              try {
                  latestGameState = decode(presentEntry.entry) as Game;
              } catch (e) { console.error("Decoding error in handleLocalGameOver:", e); }
          }
      }
      // If state couldn't be extracted, log error and exit
      if (!latestGameState) {
          errorMsg = "Could not extract or decode latest game state in handleLocalGameOver."; console.error(errorMsg, gameRecord.entry); return;
      }

      // Use the gameId prop directly as the original game hash
      const original_game_hash = gameId;
      const previous_game_hash = gameRecord.signed_action.hashed.hash; // Hash of the latest fetched action

      // --- Backend Updates ---

      // 1. Update Game Status to 'Finished' on the DHT
      try {
            // Construct the final game state object
            const finishedGameState: Game = {
                player_1: latestGameState.player_1,
                player_2: latestGameState.player_2,
                created_at: latestGameState.created_at,
                game_status: 'Finished', // Set status to Finished
                player_1_paddle: Math.round(paddle1Y), // Snapshot final positions
                player_2_paddle: Math.round(paddle2Y),
                ball_x: Math.round(ball.x),
                ball_y: Math.round(ball.y),
            };
            // Prepare the payload for the update_game zome call
            const updatePayload: UpdateGameInput = {
                 original_game_hash: original_game_hash,
                 previous_game_hash: previous_game_hash,
                 updated_game: finishedGameState,
            };
            console.log("Updating game status to Finished with payload:", updatePayload);
            // Call the backend zome function to commit the update
            await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "update_game", payload: updatePayload });
            console.log("Game status updated on DHT.");
       } catch (e) {
            console.error("Error updating game status:", e);
            errorMsg = `Failed to update game status: ${(e as Error).message}`; // Set a more informative error message
            // Ensure UI updates if errorMsg is used for display
            return; // EXIT the function if status update fails
       }

       // 2. Save Final Scores for both players on the DHT
       try {
           if (!liveGame || !liveGame.player_1) { throw new Error("liveGame or player_1 missing"); }
           // Prepare payload for Player 1's score (backend sets timestamp)
           const score1Payload: Omit<Score, 'created_at'> & { created_at?: number } = {
               game_id: original_game_hash,
               player: liveGame.player_1,
               player_points: score.player1,
           };
           await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "create_score", payload: score1Payload });
           // Prepare and send payload for Player 2's score (if P2 exists)
           if (liveGame.player_2) {
                const score2Payload: Omit<Score, 'created_at'> & { created_at?: number } = {
                   game_id: original_game_hash,
                   player: liveGame.player_2,
                   player_points: score.player2,
                };
                await client.callZome({ cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong", fn_name: "create_score", payload: score2Payload });
           }
           console.log("Scores saved.");
       } catch (e) { console.error("Error saving scores:", e); errorMsg = "Failed to save scores."; }

       // 3. Send GameOver signal using the specific function
       try {
           // Prepare payload matching backend's GameOverPayload
           const gameOverPayload = {
                game_id: original_game_hash, // Use original hash
                winner: winner, // AgentPubKey | null
                score1: score.player1,
                score2: score.player2
           };
           // Call the specific backend function to send the signal
           await client.callZome({
               cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
               fn_name: "send_game_over", // *** Use the specific function ***
               payload: gameOverPayload
            });
           console.log("GameOver signal sent.");
       } catch(e) { console.error("Error sending GameOver signal:", e); }

       // 4. (Future) Implement saving game statistics here
       // await saveStatistics();
  }

  // Handles game over triggered by receiving a GameOver signal from the opponent
  function handleRemoteGameOver(remoteWinner: AgentPubKey | null) {
      if (gameOver) return; // Prevent processing if already game over
      console.log("Handling remote game over signal...");
      gameOver = true; // Set game over flag
      winner = remoteWinner; // Store the winner received from the signal
      // The UI will update in the next 'draw' call based on the 'gameOver' flag
  }

  // --- Function to handle exit button click ---
  // Dispatches an event to App.svelte to handle navigation and state cleanup
  async function requestExit() { // Make function async
      console.log("PongGame: Requesting to abandon game and dispatching exit-game event");
      
      if (!client || !gameId) {
          console.error("PongGame: Client or gameId not available to abandon game.");
          // Still dispatch to exit UI, as backend call isn't possible
          dispatch("exit-game");
          return;
      }

      try {
          console.log(`PongGame: Calling abandon_game for gameId: ${encodeHashToBase64(gameId)}`);
          await client.callZome({
              cap_secret: null,
              role_name: HOLOCHAIN_ROLE_NAME,
              zome_name: HOLOCHAIN_ZOME_NAME,
              fn_name: "abandon_game",
              payload: gameId, // Pass the ActionHash directly
          });
          console.log("PongGame: abandon_game zome call successful.");
      } catch (e) {
          console.error("PongGame: Error calling abandon_game zome function:", e);
          // Log error, but continue to dispatch 'exit-game' to allow UI to exit
      }
      
      dispatch("exit-game"); // Dispatch the custom event
  }

  // Main canvas drawing loop, responsible for rendering the game state
  function draw() {
    // Ensure canvas context is ready
    if (!ctx) {
        // If context not ready, request next frame and exit
        // Avoid infinite loop if canvas never initializes
        if (!errorMsg) animationFrameId = requestAnimationFrame(draw);
        return;
    }

    // --- Drawing ---
    // Clear canvas and draw background/midline
    ctx.fillStyle = "#FFA500"; ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
    ctx.strokeStyle = "#000000"; ctx.lineWidth = 4; ctx.beginPath();
    ctx.setLineDash([10, 10]); ctx.moveTo(CANVAS_WIDTH / 2, 0); ctx.lineTo(CANVAS_WIDTH / 2, CANVAS_HEIGHT);
    ctx.stroke(); ctx.setLineDash([]); // Reset line dash style

    // Display Loading or Error message if game state isn't loaded yet
    // Use loadingMsg first, then errorMsg if initialization failed
    if (!liveGame && !gameOver) { // Only show loading/error if game hasn't started or finished
        ctx.fillStyle = "#000000"; ctx.font = "30px 'Press Start 2P', monospace"; ctx.textAlign = "center";
        ctx.fillText(errorMsg || loadingMsg || "Loading...", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2);
        // Keep requesting frames only if still loading (no error and not game over)
        if (!errorMsg && loadingMsg) animationFrameId = requestAnimationFrame(draw);
        return; // Don't draw game elements if not loaded/ready
    }

    // Draw Game Elements (only if liveGame is set)
    if (liveGame) {
        ctx.fillStyle = "#000000";
        ctx.fillRect(0, paddle1Y, PADDLE_WIDTH, PADDLE_HEIGHT); // Player 1 Paddle (left)
        ctx.fillRect(CANVAS_WIDTH - PADDLE_WIDTH, paddle2Y, PADDLE_WIDTH, PADDLE_HEIGHT); // Player 2 Paddle (right)
        ctx.beginPath(); ctx.arc(ball.x, ball.y, BALL_RADIUS, 0, 2 * Math.PI); ctx.fill(); // Ball

        // Draw Scores
        ctx.font = "40px 'Press Start 2P', monospace"; ctx.textAlign = "center";
        ctx.fillText(score.player1.toString(), CANVAS_WIDTH / 4, 60); // Player 1 Score
        ctx.fillText(score.player2.toString(), (3 * CANVAS_WIDTH) / 4, 60); // Player 2 Score
    }

    // --- Game Over Overlay ---
    // Display if the gameOver flag is true
    if (gameOver) {
        ctx.fillStyle = "rgba(0, 0, 0, 0.7)"; ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT); // Dim background
        ctx.fillStyle = "#000000"; ctx.font = "50px 'Press Start 2P', monospace"; ctx.textAlign = "center";
        ctx.fillText("GAME OVER", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2 - 50);
        ctx.font = "30px 'Press Start 2P', monospace";
         // Display winner's name
         if (winner && liveGame) {
             const winnerName = encodeHashToBase64(winner) === encodeHashToBase64(liveGame.player_1) ? "Player 1" : "Player 2";
             ctx.fillText(`${winnerName} Wins!`, CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2);
         } else { ctx.fillText("Game Finished", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2); } // Fallback if no winner determined
         // Display final score
         ctx.font = "40px 'Press Start 2P', monospace";
         ctx.fillText(`${score.player1} - ${score.player2}`, CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2 + 50);
        // Stop the animation loop once the game over screen is drawn
        return;
    }

    // --- Game Logic & Next Frame Scheduling ---
    // Update ball physics and score (only Player 1)
    if (liveGame && liveGame.game_status === 'InProgress') { // Only run physics if game is InProgress
        if (isPlayer1) updateBallAndScore();
        animationFrameId = requestAnimationFrame(draw); // Continue loop
    } else if (liveGame && liveGame.game_status === 'Waiting') {
        // If somehow we are drawing but status is still Waiting, show message and wait
        ctx.fillStyle = "#888"; ctx.font = "24px Arial"; ctx.textAlign = "center";
        ctx.fillText("Waiting for game to start...", CANVAS_WIDTH / 2, CANVAS_HEIGHT - 50);
        animationFrameId = requestAnimationFrame(draw); // Continue loop while waiting
    }
  }

  // --- Component Lifecycle ---
  onMount(async () => {
    client = await appClientContext.getClient(); // Initialize Holochain client
    if (canvas) {
        ctx = canvas.getContext("2d")!;
    } else {
        console.error("Canvas element not found on mount.");
        errorMsg = "Failed to initialize canvas.";
        // Attempt to draw error even without game loop starting
        if(ctx) draw();
        return; // Stop initialization if canvas fails
    }
    // Start the initialization process (which includes retries)
    initializeGame();
  });

  onDestroy(() => {
    // Cleanup logic when the component is removed from the DOM
    console.log("PongGame component destroyed. Cleaning up...");
    // Clear any pending retry timeouts
    if (retryTimeoutId) clearTimeout(retryTimeoutId);
    // Stop animation loop and remove listeners
    cancelAnimationFrame(animationFrameId);
    window.removeEventListener("keydown", handleKeyDown);
    if (unsubscribeFromSignals) unsubscribeFromSignals(); // Unsubscribe from Holochain signals
  });

</script>

<div class="game-container">
    {#if errorMsg && !ctx} <p class="error-message">Error: {errorMsg}</p> {/if}

    <div class="game-window">
        <div class="players-info">
            <div class="player player1">P1: {#if player1Profile?.nickname}{player1Profile.nickname}{:else if liveGame?.player_1}{truncatePubkey(liveGame.player_1)}{:else}Loading...{/if}</div>
            <div class="player player2">P2: {#if player2Profile?.nickname}{player2Profile.nickname}{:else if liveGame?.player_2}{truncatePubkey(liveGame.player_2)}{:else}Waiting...{/if}</div>
        </div>

        <canvas bind:this={canvas} width={CANVAS_WIDTH} height={CANVAS_HEIGHT}></canvas>

        {#if gameOver}
            <div class="game-over-menu">
                <button on:click={requestExit}>Back to Lobby</button>
            </div>
        {:else if liveGame || errorMsg}
            <div class="exit-game-button">
                 <button on:click={requestExit}>Exit Game</button>
            </div>
        {/if}
    </div>
</div>

<style>
  .game-container { display: flex; justify-content: center; align-items: center; flex-direction: column; padding-top: 20px; }
  .error-message { color: red; margin-bottom: 10px; font-weight: bold; }
  .game-window { position: relative; }
  .players-info {
    position: absolute;
    top: -25px; 
    left: 0;
    width: 100%;
    display: flex;
    justify-content: space-between;
    padding: 0 15px; 
    box-sizing: border-box; 
    color: orange;
    font-size: 0.9rem;
    font-weight: bold;
    z-index: 1; 
    pointer-events: none; 
  }
  .player { background-color: rgba(0,0,0,0.6); padding: 3px 6px; border-radius: 4px; }
  canvas {
    background-color: orange;
    display: block; 
    margin: 0 auto; 
    border: 3px solid black; 
    box-shadow: none;
  }
  .exit-game-button {
    position: absolute;
    top: 10px;
    right: 10px; 
    z-index: 10;
  }
  .exit-game-button button {
    font-size: 0.9rem;
    padding: 0.4rem 0.8rem;
    background-color: orange; 
    color:black;
    border: none;
    border-radius: 5px;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }
   .exit-game-button button:hover { background-color: red; }

  .game-over-menu {
    position: absolute;
    bottom: 30px; 
    left: 50%;
    transform: translateX(-50%); 
    z-index: 10;
  }
  .game-over-menu button {
    font-size: 1.2rem;
    padding: 0.8rem 1.5rem;
    background-color: orange; 
    color: black;
    border: none;
    border-radius: 5px;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }
  .game-over-menu button:hover { background-color: red; }
</style>
