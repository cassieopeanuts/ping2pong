// Web Audio API Retro Synth Sound Effects
// Dynamic sound synthesis requiring zero asset files

let audioCtx: AudioContext | null = null;

function getAudioContext(): AudioContext {
  if (!audioCtx) {
    // Create new AudioContext (supported in modern browsers)
    audioCtx = new (window.AudioContext || (window as any).webkitAudioContext)();
  }
  // Resume context if suspended (browser security autoplays requirement)
  if (audioCtx.state === "suspended") {
    audioCtx.resume();
  }
  return audioCtx;
}

/**
 * Play a classic 8-bit square wave bleep for paddle hit
 */
export function playPaddleHit() {
  try {
    const ctx = getAudioContext();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.connect(gain);
    gain.connect(ctx.destination);

    // Classic square wave retro sound
    osc.type = "square";
    
    // Frequency sweep down quickly
    const now = ctx.currentTime;
    osc.frequency.setValueAtTime(600, now);
    osc.frequency.exponentialRampToValueAtTime(150, now + 0.1);

    // Gain envelope (decay quickly)
    gain.gain.setValueAtTime(0.1, now);
    gain.gain.exponentialRampToValueAtTime(0.01, now + 0.1);

    osc.start(now);
    osc.stop(now + 0.12);
  } catch (e) {
    console.warn("Web Audio API not supported or suspended:", e);
  }
}

/**
 * Play a low triangle wave thud for wall bounces
 */
export function playWallBounce() {
  try {
    const ctx = getAudioContext();
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.connect(gain);
    gain.connect(ctx.destination);

    // Triangle wave has a softer, rounder thud sound
    osc.type = "triangle";
    
    const now = ctx.currentTime;
    osc.frequency.setValueAtTime(120, now);
    osc.frequency.linearRampToValueAtTime(60, now + 0.08);

    gain.gain.setValueAtTime(0.15, now);
    gain.gain.exponentialRampToValueAtTime(0.01, now + 0.08);

    osc.start(now);
    osc.stop(now + 0.09);
  } catch (e) {
    console.warn("Web Audio API error:", e);
  }
}

/**
 * Play a quick ascending square wave arpeggio on point scored
 */
export function playPointScored() {
  try {
    const ctx = getAudioContext();
    const now = ctx.currentTime;

    const notes = [261.63, 329.63, 392.00, 523.25]; // C4, E4, G4, C5 arpeggio
    const noteLength = 0.07;

    notes.forEach((freq, idx) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();

      osc.connect(gain);
      gain.connect(ctx.destination);

      osc.type = "square";
      osc.frequency.setValueAtTime(freq, now + idx * noteLength);

      gain.gain.setValueAtTime(0.08, now + idx * noteLength);
      gain.gain.exponentialRampToValueAtTime(0.005, now + idx * noteLength + noteLength);

      osc.start(now + idx * noteLength);
      osc.stop(now + idx * noteLength + noteLength);
    });
  } catch (e) {
    console.warn("Web Audio API error:", e);
  }
}

/**
 * Play a fun descending minor arpeggio when game is over
 */
export function playGameOver() {
  try {
    const ctx = getAudioContext();
    const now = ctx.currentTime;

    const notes = [587.33, 493.88, 440.00, 349.23, 293.66]; // D5, B4, A4, F4, D4
    const noteLength = 0.12;

    notes.forEach((freq, idx) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();

      osc.connect(gain);
      gain.connect(ctx.destination);

      osc.type = "sawtooth"; // Raspier retro feel
      osc.frequency.setValueAtTime(freq, now + idx * noteLength);

      gain.gain.setValueAtTime(0.08, now + idx * noteLength);
      gain.gain.exponentialRampToValueAtTime(0.005, now + idx * noteLength + noteLength * 1.5);

      osc.start(now + idx * noteLength);
      osc.stop(now + idx * noteLength + noteLength * 1.5);
    });
  } catch (e) {
    console.warn("Web Audio API error:", e);
  }
}
