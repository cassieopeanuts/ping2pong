<script lang="ts">
  import { onMount, onDestroy, getContext, createEventDispatcher } from "svelte";
  // Import types from holochain/client
  import type { AppClient, ActionHash, AgentPubKey, Record, Entry } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  // Import local types and context
  import { clientContext, type ClientContext } from "../../contexts";
  import { decode } from "@msgpack/msgpack";
  // Import local types including the specific signal structures if needed for receiving
  // Note: Signal types are used here for clarity but aren't strictly required if only checking `signalPayload.type`
  import type { Game, Score, GameStatus, UpdateGameInput, PaddleUpdateSignal, BallUpdateSignal, GameOverSignal, ScoreUpdateSignal } from "../ping_2_pong/types";
  import { getOrFetchProfile, type DisplayProfile } from "../../stores/profilesStore";
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "../../holochainConfig";
  import { playPaddleHit, playWallBounce, playPointScored, playGameOver } from "../../utils/audio";

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
  const UPDATE_INTERVAL = 20; // ms interval for sending signal updates (50 updates/sec)

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

  // Retro Particle Effect & Screenshake
  interface Particle {
    x: number;
    y: number;
    vx: number;
    vy: number;
    size: number;
    color: string;
    life: number; // 1 to 0
    decay: number;
  }
  let particles: Particle[] = [];
  let shakeAmt = 0;

  function createExplosion(x: number, y: number, count: number = 15, isScore: boolean = false) {
    const colors = isScore ? ["#FFA500", "#FFC400", "#FFD700", "#000000"] : ["#000000", "#FFA500", "#FFFFFF"];
    for (let i = 0; i < count; i++) {
      const angle = Math.random() * Math.PI * 2;
      const speed = Math.random() * (isScore ? 8 : 4) + (isScore ? 2 : 1);
      particles.push({
        x,
        y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        size: Math.random() * (isScore ? 6 : 4) + 2,
        color: colors[Math.floor(Math.random() * colors.length)],
        life: 1.0,
        decay: Math.random() * 0.05 + 0.02
      });
    }
    particles = [...particles];
  }

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
      if (!isMounted) return;
      console.log(`[PongGame initializeGame] Starting initialization. Retry count: ${retryCount}`);
      loadingMsg = `Initializing game... (Attempt ${retryCount + 1})`;
      errorMsg = null; // Clear previous errors

      const fetchedGame = await fetchGameState();
      if (!isMounted) return;

      if (fetchedGame) {
          // --- Game Ready ---
          loadingMsg = null; // Clear loading message
          liveGame = fetchedGame; // Set the live game state

          function getPubkeyStr(pk: any): string {
              if (!pk) return "";
              if (typeof pk === "string") return pk;
              try { return encodeHashToBase64(pk); } catch { return String(pk); }
          }

          // Identify players based on the confirmed state (using client.myPubKey as absolute authority)
          const myKey = client?.myPubKey || playerKey;
          const myPubKeyB64 = getPubkeyStr(myKey);
          const p1B64 = getPubkeyStr(liveGame.player_1);
          const p2B64 = getPubkeyStr(liveGame.player_2);

          isPlayer1 = (p1B64 === myPubKeyB64);
          isPlayer2 = (p2B64 === myPubKeyB64);

          // Fallback if formatting differed: creator is P1, joiner is P2
          if (!isPlayer1 && !isPlayer2) {
              if (liveGame.player_2) {
                  isPlayer2 = true;
              } else {
                  isPlayer1 = true;
              }
          }
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
              ball.dx = 2.5 * (Math.random() > 0.5 ? 1 : -1);
              ball.dy = 2.0 * (Math.random() > 0.5 ? 1 : -1);
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

  let activeKeys = new Set<string>();

  // Starts the main game loop and sets up listeners
  function startGameLoop() {
      if (!isMounted) return;
      if (!ctx) {
          console.error("[PongGame startGameLoop] Canvas context not available!");
          errorMsg = "Canvas failed to initialize.";
          return;
      }
      if (animationFrameId) {
          cancelAnimationFrame(animationFrameId);
      }
      if (unsubscribeFromSignals) {
          unsubscribeFromSignals();
          unsubscribeFromSignals = undefined;
      }
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);

      console.log("[PongGame startGameLoop] Starting game loop and listeners.");
      gameOver = false; // Ensure game isn't marked over
      draw(); // Start drawing loop
      window.addEventListener("keydown", handleKeyDown); // Listen for keyboard input
      window.addEventListener("keyup", handleKeyUp);
      unsubscribeFromSignals = subscribeToGameSignals(); // Subscribe to game signals
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (gameOver || !liveGame) return;
    if (["ArrowUp", "ArrowDown", "KeyW", "KeyS", "w", "W", "s", "S"].includes(e.key) || ["ArrowUp", "ArrowDown", "KeyW", "KeyS"].includes(e.code)) {
        e.preventDefault();
    }
    activeKeys.add(e.key);
    activeKeys.add(e.code);
    updatePaddleInput();
  }

  function handleKeyUp(e: KeyboardEvent) {
    activeKeys.delete(e.key);
    activeKeys.delete(e.code);
  }

  let lastSentPaddleY: number | null = null;

  // Updates paddle position based on keyboard input
  function updatePaddleInput() {
    if (gameOver || !liveGame) return;

    const isUp = activeKeys.has("ArrowUp") || activeKeys.has("w") || activeKeys.has("W") || activeKeys.has("KeyW");
    const isDown = activeKeys.has("ArrowDown") || activeKeys.has("s") || activeKeys.has("S") || activeKeys.has("KeyS");

    let moved = false;
    if (isPlayer1) {
      if (isUp) {
        paddle1Y = Math.max(0, paddle1Y - PADDLE_SPEED);
        moved = true;
      } else if (isDown) {
        paddle1Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle1Y + PADDLE_SPEED);
        moved = true;
      }
    } else if (isPlayer2) {
      if (isUp) {
        paddle2Y = Math.max(0, paddle2Y - PADDLE_SPEED);
        moved = true;
      } else if (isDown) {
        paddle2Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle2Y + PADDLE_SPEED);
        moved = true;
      }
    }

    const currentY = Math.round(isPlayer1 ? paddle1Y : paddle2Y);
    if (moved) {
      sendPaddleUpdate();
    } else if (lastSentPaddleY !== null && lastSentPaddleY !== currentY) {
      sendPaddleUpdate(true);
    }
  }

  $: opponentKey = isPlayer1 ? liveGame?.player_2 : liveGame?.player_1;

  // Sends the current player's paddle position update signal
  async function sendPaddleUpdate(force = false) {
    const now = Date.now();
    const currentY = Math.round(isPlayer1 ? paddle1Y : paddle2Y);

    if (gameOver || !client || !liveGame || !gameId || !opponentKey) return;
    if (!force && lastSentPaddleY === currentY) return;
    if (!force && (now - lastPaddleUpdate < UPDATE_INTERVAL)) return;

    lastPaddleUpdate = now;
    lastSentPaddleY = currentY;

    // Prepare payload matching the backend's PaddleUpdatePayload struct
    const payload = {
        game_id: gameId, // The original ActionHash identifying the game
        recipient: opponentKey ?? null,
        paddle_y: currentY
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
    if (gameOver || !isPlayer1 || !client || !liveGame || !gameId || !opponentKey || (now - lastBallUpdate < UPDATE_INTERVAL)) return;
    lastBallUpdate = now; // Update timestamp

    // Prepare payload matching the backend's BallUpdatePayload struct
    const payload = {
        game_id: gameId, // The original ActionHash identifying the game
        recipient: opponentKey ?? null,
        ball_x: Math.round(ball.x),
        ball_y: Math.round(ball.y),
        ball_dx: Math.round(ball.dx),
        ball_dy: Math.round(ball.dy),
        score1: score.player1,
        score2: score.player2
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
          recipient: opponentKey ?? null,
          score1 : score.player1,
          score2 : score.player2
        }
      });
    } catch (e) { console.error("Score update failed:", e); }
  }

  // Sets up the listener for incoming signals related to this specific game
  function subscribeToGameSignals() {
    if (!isMounted || !client) return;

    return client.on("signal", (raw: any) => {
      if (!isMounted) return;
      let s = raw;
      if (raw?.App?.payload) s = raw.App.payload;
      else if (raw?.value?.payload) s = raw.value.payload;
      else if (raw?.payload) s = raw.payload;

      if (!s || !s.type || gameOver) return;
      if (s.game_id && encodeHashToBase64(s.game_id) !== encodeHashToBase64(gameId)) return;

      const meB64 = encodeHashToBase64(playerKey);

      try {
        switch (s.type) {
          case "PaddleUpdate":
            if (encodeHashToBase64(s.player) !== meB64) {
              if (isPlayer1) paddle2Y = s.paddle_y;
              else           paddle1Y = s.paddle_y;
            }
            break;

          case "PaddleHit":
            if (encodeHashToBase64(s.player) !== meB64) {
              const boost = Math.min(12, Math.abs(ball.dx) * 1.08);
              ball.dx = isPlayer1 ? -boost : boost; // Deflect away from hitting player
              ball.y = s.ball_y;
              playPaddleHit();
              shakeAmt = 8;
              createExplosion(ball.x, ball.y, 8, false);
            }
            break;

          case "BallUpdate":
            if (!isPlayer1) {
              const oldDx = ball.dx;
              const oldDy = ball.dy;
              ball.x = s.ball_x; ball.y = s.ball_y;
              ball.dx = s.ball_dx; ball.dy = s.ball_dy;
              if (s.score1 !== undefined && s.score1 !== null && s.score2 !== undefined && s.score2 !== null) {
                score = { player1: s.score1, player2: s.score2 };
              }
              
              if (Math.sign(oldDx) !== Math.sign(ball.dx) && oldDx !== 0) {
                playPaddleHit();
                shakeAmt = 8;
                createExplosion(ball.x, ball.y, 8, false);
              } else if (Math.sign(oldDy) !== Math.sign(ball.dy) && oldDy !== 0) {
                playWallBounce();
              }
            }
            break;

          case "ScoreUpdate":
            if (!isPlayer1) {
              playPointScored();
              shakeAmt = 20;
              createExplosion(ball.x, ball.y, 25, true);
            }
            score = { player1: s.score1, player2: s.score2 };
            break;

          case "GameOver":
            playGameOver();
            if (s.score1 !== undefined && s.score1 !== null && s.score2 !== undefined && s.score2 !== null) {
              score = { player1: s.score1, player2: s.score2 };
            }
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
      playWallBounce();
    }

    // Check for collisions with paddles (with 18px latency compensation margin)
    let hitPaddle = false;
    const HITBOX_GRACE = 18; // Compensate for P2P network latency

    // Player 1 paddle collision logic
    if (ball.dx < 0 && ball.x - BALL_RADIUS <= PADDLE_WIDTH + 8 && ball.x >= BALL_RADIUS - 10 && ball.y >= paddle1Y - HITBOX_GRACE && ball.y <= paddle1Y + PADDLE_HEIGHT + HITBOX_GRACE) {
        const speedBoost = Math.min(12, Math.abs(ball.dx) * 1.08);
        ball.dx = speedBoost; // Reverse horizontal velocity and accelerate
        ball.x = PADDLE_WIDTH + BALL_RADIUS; // Reposition ball
        ball.dy = (ball.y - (paddle1Y + PADDLE_HEIGHT / 2)) * 0.35; // Add vertical angle
        hitPaddle = true;
        playPaddleHit();
        shakeAmt = 8;
        createExplosion(ball.x, ball.y, 8, false);
    }
    // Player 2 paddle collision logic
    else if (ball.dx > 0 && ball.x + BALL_RADIUS >= CANVAS_WIDTH - PADDLE_WIDTH - 8 && ball.x <= CANVAS_WIDTH - BALL_RADIUS + 10 && ball.y >= paddle2Y - HITBOX_GRACE && ball.y <= paddle2Y + PADDLE_HEIGHT + HITBOX_GRACE) {
        const speedBoost = Math.min(12, Math.abs(ball.dx) * 1.08);
        ball.dx = -speedBoost; // Reverse horizontal velocity and accelerate
        ball.x = CANVAS_WIDTH - PADDLE_WIDTH - BALL_RADIUS; // Reposition ball
        ball.dy = (ball.y - (paddle2Y + PADDLE_HEIGHT / 2)) * 0.35; // Add vertical angle
        hitPaddle = true;
        playPaddleHit();
        shakeAmt = 8;
        createExplosion(ball.x, ball.y, 8, false);
    }

    // Check if a player scored (ball went past a paddle)
    let scored = false;
    if (ball.x + BALL_RADIUS < 0) {          // P2 scores
      score = { player1: score.player1, player2: score.player2 + 1 };
      scored = true;
      sendScoreUpdate();
    } else if (ball.x - BALL_RADIUS > CANVAS_WIDTH) { // P1 scores
      score = { player1: score.player1 + 1, player2: score.player2 };
      scored = true;
      sendScoreUpdate();
    }

    // Handle the outcome of the physics update
    if (scored) {
      console.log(`Score: ${score.player1} - ${score.player2}`);
      playPointScored();
      shakeAmt = 20;
      createExplosion(ball.x, ball.y, 25, true);

      // Check if the game has been won
      if (score.player1 >= WINNING_SCORE || score.player2 >= WINNING_SCORE) {
        winner = score.player1 >= WINNING_SCORE ? liveGame.player_1 : liveGame.player_2; // Determine winner
        gameOver = true; // Set game over flag
        if(winner) console.log("Game Over! Winner:", truncatePubkey(winner));
        handleLocalGameOver(); // Trigger backend updates and game over signal
      } else {
        // If game not over, reset ball for the next point with gentle serve speed
        ball.x = CANVAS_WIDTH / 2;
        ball.y = CANVAS_HEIGHT / 2;
        ball.dx = 2.5 * (score.player1 > score.player2 ? -1 : 1); // Serve towards player who lost point
        ball.dy = 2.0 * (Math.random() > 0.5 ? 1 : -1); // Gentle vertical serve direction
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
    playGameOver();
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

      // 1. Update Game Status to 'Finished' on the DHT via finish_game
      try {
            console.log("Finishing game status on DHT...");
            await client.callZome({
                cap_secret: null,
                role_name: "ping_2_pong",
                zome_name: "ping_2_pong",
                fn_name: "finish_game",
                payload: {
                    original_game_hash: original_game_hash,
                    previous_game_hash: previous_game_hash
                }
            });
            console.log("Game status updated to Finished on DHT.");
       } catch (e) {
            console.error("Error finishing game status:", e);
            // Proceed to save scores even if status link had a minor issue
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
                recipient: opponentKey ?? null,
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

  // --- NEW: Function to handle exit button click ---
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
    if (!isMounted) return;
    // Ensure canvas context is ready
    if (!ctx) {
        // If context not ready, request next frame and exit
        // Avoid infinite loop if canvas never initializes
        if (!errorMsg) animationFrameId = requestAnimationFrame(draw);
        return;
    }

    // Save context for screenshake
    ctx.save();
    if (shakeAmt > 0) {
      const dx = (Math.random() - 0.5) * shakeAmt;
      const dy = (Math.random() - 0.5) * shakeAmt;
      ctx.translate(dx, dy);
      shakeAmt -= 0.8;
      if (shakeAmt < 0) shakeAmt = 0;
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
        ctx.restore();
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

        // Draw active particles
        particles = particles.filter(p => {
          p.x += p.vx;
          p.y += p.vy;
          p.life -= p.decay;
          if (p.life <= 0) return false;
          
          ctx.save();
          ctx.globalAlpha = p.life;
          ctx.fillStyle = p.color;
          ctx.fillRect(p.x - p.size / 2, p.y - p.size / 2, p.size, p.size);
          ctx.restore();
          return true;
        });

        // Draw Scores
        ctx.fillStyle = "#000000";
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
        ctx.restore();
        // Stop the animation loop once the game over screen is drawn
        return;
    }

    // --- Game Logic & Next Frame Scheduling ---
    // Update paddle movement continuously and ball physics (only Player 1)
    if (liveGame && liveGame.game_status === 'InProgress') { // Only run physics if game is InProgress
        updatePaddleInput();
        if (isPlayer1) updateBallAndScore();
        ctx.restore();
        animationFrameId = requestAnimationFrame(draw); // Continue loop
    } else if (liveGame && liveGame.game_status === 'Waiting') {
        // If somehow we are drawing but status is still Waiting, show message and wait
        ctx.fillStyle = "#888"; ctx.font = "24px Arial"; ctx.textAlign = "center";
        ctx.fillText("Waiting for game to start...", CANVAS_WIDTH / 2, CANVAS_HEIGHT - 50);
        ctx.restore();
        animationFrameId = requestAnimationFrame(draw); // Continue loop while waiting
    } else {
        ctx.restore();
    }
  }

  let isMounted = true;

  // --- Component Lifecycle ---
  onMount(async () => {
    const fetchedClient = await appClientContext.getClient(); // Initialize Holochain client
    if (!isMounted) return;
    client = fetchedClient;
    if (canvas) {
        ctx = canvas.getContext("2d")!;
    } else {
        console.error("Canvas element not found on mount.");
        errorMsg = "Failed to initialize canvas.";
        // Attempt to draw error even without game loop starting
        if(ctx && isMounted) draw();
        return; // Stop initialization if canvas fails
    }
    // Start the initialization process (which includes retries)
    initializeGame();
  });

  onDestroy(() => {
    isMounted = false;
    // Cleanup logic when the component is removed from the DOM
    console.log("PongGame component destroyed. Cleaning up...");
    // Clear any pending retry timeouts
    if (retryTimeoutId) clearTimeout(retryTimeoutId);
    // Stop animation loop and remove listeners
    cancelAnimationFrame(animationFrameId);
    window.removeEventListener("keydown", handleKeyDown);
    window.removeEventListener("keyup", handleKeyUp);
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
  .game-container { display: flex; justify-content: center; align-items: center; flex-direction: column; padding-top: 20px; width: 100%; box-sizing: border-box; }
  .error-message { color: red; margin-bottom: 10px; font-weight: bold; }
  .game-window { position: relative; width: 100%; max-width: 800px; margin: 0 auto; box-sizing: border-box; }
  .players-info {
    position: absolute;
    top: -25px; /* Position above the canvas */
    left: 0;
    width: 100%;
    display: flex;
    justify-content: space-between;
    padding: 0 15px; /* Padding on the sides */
    box-sizing: border-box; /* Include padding in width calculation */
    color: orange;
    font-size: 0.9rem;
    font-weight: bold;
    z-index: 1; /* Ensure it's above the canvas */
    pointer-events: none; /* Prevent interaction */
  }
  .player { background-color: rgba(0,0,0,0.85); padding: 3px 6px; border-radius: 0px; border: 2px solid var(--primary-text-color); }
  canvas {
    background-color: orange;
    display: block; /* Remove extra space below canvas */
    margin: 0 auto; /* Center canvas */
    border: 3px solid black; /* Border around the game area */
    box-shadow: none;
    width: 100%;
    max-width: 800px;
    height: auto;
    aspect-ratio: 4 / 3;
    box-sizing: border-box;
  }
  .exit-game-button {
    position: absolute;
    top: 10px; /* Position near the top */
    right: 10px; /* Position on the right */
    z-index: 10;
  }
  .exit-game-button button {
    font-size: 0.8rem;
    font-family: inherit;
    padding: 0.4rem 0.8rem;
    background-color: var(--primary-text-color);
    color: var(--primary-bg-color);
    border: 2px solid var(--primary-bg-color);
    border-radius: 0px;
    cursor: pointer;
    transition: background-color 0.2s ease, color 0.2s ease, border-color 0.2s ease;
  }
  .exit-game-button button:hover {
    background-color: var(--primary-bg-color);
    color: var(--primary-text-color);
    border-color: var(--primary-text-color);
  }

  .game-over-menu {
    position: absolute;
    bottom: 30px; /* Position towards the bottom */
    left: 50%;
    transform: translateX(-50%); /* Center horizontally */
    z-index: 10;
  }
  .game-over-menu button {
    font-size: 1rem;
    font-family: inherit;
    padding: 0.8rem 1.5rem;
    background-color: var(--primary-text-color);
    color: var(--primary-bg-color);
    border: 3px solid var(--primary-bg-color);
    border-radius: 0px;
    cursor: pointer;
    transition: background-color 0.2s ease, color 0.2s ease, border-color 0.2s ease;
  }
  .game-over-menu button:hover {
    background-color: var(--primary-bg-color);
    color: var(--primary-text-color);
    border-color: var(--primary-text-color);
  }

</style>
