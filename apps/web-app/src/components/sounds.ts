// Sound utilities - pure functions and event-based system
// Sounds are triggered via custom events, no UI components

// ============================================
// Audio Context Management
// ============================================
let sharedAudioContext: AudioContext | null = null;

function getAudioContext(): AudioContext {
  if (!sharedAudioContext || sharedAudioContext.state === "closed") {
    sharedAudioContext = new AudioContext();
  }
  if (sharedAudioContext.state === "suspended") {
    sharedAudioContext.resume();
  }
  return sharedAudioContext;
}

// ============================================
// Sound Event Types
// ============================================
export type SoundType =
  | "ui-click"
  | "success-chime"
  | "error-tone"
  | "notification-ding"
  | "whoosh"
  | "confetti"
  | "error-file"
  | "firework"
  | "magic"
  | "success-file"
  | "warning";

export interface SoundEventDetail {
  volume?: number;
}

// Custom event name
export const SOUND_EVENT = "play-sound";

// ============================================
// Sound Playback Functions
// ============================================

export function playUIClick(volume = 0.3): void {
  const ctx = getAudioContext();
  const now = ctx.currentTime;

  const osc = ctx.createOscillator();
  const gain = ctx.createGain();

  osc.type = "sine";
  osc.frequency.setValueAtTime(1800, now);
  osc.frequency.exponentialRampToValueAtTime(1200, now + 0.03);

  gain.gain.setValueAtTime(0, now);
  gain.gain.linearRampToValueAtTime(volume, now + 0.002);
  gain.gain.exponentialRampToValueAtTime(0.001, now + 0.05);

  osc.connect(gain);
  gain.connect(ctx.destination);

  osc.start(now);
  osc.stop(now + 0.05);
}

export function playSuccessChime(volume = 0.25): void {
  const ctx = getAudioContext();
  const now = ctx.currentTime;

  const notes = [784, 1047]; // G5, C6

  notes.forEach((freq, i) => {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.value = freq;

    const startTime = now + i * 0.12;
    gain.gain.setValueAtTime(0, startTime);
    gain.gain.linearRampToValueAtTime(volume, startTime + 0.01);
    gain.gain.setValueAtTime(volume, startTime + 0.08);
    gain.gain.exponentialRampToValueAtTime(0.001, startTime + 0.35);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(startTime);
    osc.stop(startTime + 0.35);
  });
}

export function playErrorTone(volume = 0.2): void {
  const ctx = getAudioContext();
  const now = ctx.currentTime;

  const notes = [440, 349]; // A4, F4

  notes.forEach((freq, i) => {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "triangle";
    osc.frequency.value = freq;

    const startTime = now + i * 0.1;
    gain.gain.setValueAtTime(0, startTime);
    gain.gain.linearRampToValueAtTime(volume, startTime + 0.005);
    gain.gain.setValueAtTime(volume, startTime + 0.06);
    gain.gain.exponentialRampToValueAtTime(0.001, startTime + 0.15);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(startTime);
    osc.stop(startTime + 0.15);
  });
}

export function playNotificationDing(volume = 0.25): void {
  const ctx = getAudioContext();
  const now = ctx.currentTime;

  const fundamental = 880; // A5
  const harmonics = [1, 2, 3, 4.2];
  const amplitudes = [1, 0.5, 0.25, 0.125];

  harmonics.forEach((h, i) => {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();

    osc.type = "sine";
    osc.frequency.value = fundamental * h;

    const amp = volume * amplitudes[i];
    gain.gain.setValueAtTime(0, now);
    gain.gain.linearRampToValueAtTime(amp, now + 0.003);
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.4 - i * 0.05);

    osc.connect(gain);
    gain.connect(ctx.destination);

    osc.start(now);
    osc.stop(now + 0.5);
  });
}

export function playWhoosh(volume = 0.15): void {
  const ctx = getAudioContext();
  const now = ctx.currentTime;
  const duration = 0.25;

  const bufferSize = ctx.sampleRate * duration;
  const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
  const data = buffer.getChannelData(0);

  for (let i = 0; i < bufferSize; i++) {
    const t = i / bufferSize;
    const envelope = Math.sin(Math.PI * t) * Math.pow(1 - t, 0.5);
    data[i] = (Math.random() * 2 - 1) * envelope;
  }

  const noise = ctx.createBufferSource();
  noise.buffer = buffer;

  const filter = ctx.createBiquadFilter();
  filter.type = "bandpass";
  filter.Q.value = 1.5;
  filter.frequency.setValueAtTime(300, now);
  filter.frequency.exponentialRampToValueAtTime(2000, now + duration * 0.4);
  filter.frequency.exponentialRampToValueAtTime(800, now + duration);

  const gain = ctx.createGain();
  gain.gain.value = volume;

  noise.connect(filter);
  filter.connect(gain);
  gain.connect(ctx.destination);

  noise.start(now);
}

// ============================================
// File-based Sound Players
// ============================================

function getSoundUrl(soundFile: string): string {
  const canPlayOgg = document.createElement("audio").canPlayType("audio/ogg");
  const ext = canPlayOgg ? "ogg" : "mp3";
  return `/app/sounds/${soundFile}.${ext}`;
}

function playFileSound(soundFile: string, volume = 0.5): void {
  const audio = new Audio(getSoundUrl(soundFile));
  audio.volume = volume;
  audio.play().catch(() => {
    // Silently fail if audio can't play
  });
}

export function playConfetti(volume = 0.5): void {
  playFileSound("confeti", volume);
}

export function playErrorFile(volume = 0.5): void {
  playFileSound("error", volume);
}

export function playFirework(volume = 0.5): void {
  playFileSound("firework", volume);
}

export function playMagic(volume = 0.5): void {
  playFileSound("magic", volume);
}

export function playSuccessFile(volume = 0.5): void {
  playFileSound("success", volume);
}

export function playWarning(volume = 0.5): void {
  playFileSound("warning", volume);
}

// ============================================
// Sound Player Map
// ============================================

const soundPlayers: Record<SoundType, (volume?: number) => void> = {
  "ui-click": playUIClick,
  "success-chime": playSuccessChime,
  "error-tone": playErrorTone,
  "notification-ding": playNotificationDing,
  "whoosh": playWhoosh,
  "confetti": playConfetti,
  "error-file": playErrorFile,
  "firework": playFirework,
  "magic": playMagic,
  "success-file": playSuccessFile,
  "warning": playWarning,
};

// ============================================
// Event-based Sound Trigger
// ============================================

export function triggerSound(sound: SoundType, volume?: number): void {
  const player = soundPlayers[sound];
  if (player) {
    player(volume);
  }
}

// Dispatch a sound event (for components to use)
export function dispatchSoundEvent(
  element: HTMLElement,
  sound: SoundType,
  volume?: number
): void {
  element.dispatchEvent(
    new CustomEvent(SOUND_EVENT, {
      bubbles: true,
      composed: true,
      detail: { sound, volume } as SoundEventDetail & { sound: SoundType },
    })
  );
}

// ============================================
// Sound Listener Mixin/Setup
// ============================================

export interface SoundListenerHost {
  addEventListener(
    type: string,
    listener: EventListener,
    options?: boolean | AddEventListenerOptions
  ): void;
  removeEventListener(
    type: string,
    listener: EventListener,
    options?: boolean | EventListenerOptions
  ): void;
}

export function setupSoundListener(host: SoundListenerHost): () => void {
  const handler = (event: Event) => {
    const customEvent = event as CustomEvent<
      SoundEventDetail & { sound: SoundType }
    >;
    const { sound, volume } = customEvent.detail;
    triggerSound(sound, volume);
  };

  host.addEventListener(SOUND_EVENT, handler);

  // Return cleanup function
  return () => host.removeEventListener(SOUND_EVENT, handler);
}

// ============================================
// LitElement Controller for Sound Listening
// ============================================

// Simple controller that integrates with LitElement lifecycle
export class SoundController {
  private cleanup?: () => void;

  constructor(host: HTMLElement & { addController?: (controller: unknown) => void }) {
    // Hook into LitElement lifecycle if available
    if (host.addController) {
      host.addController(this);
    }
  }

  hostConnected(): void {
    // No-op: we use direct play() calls instead of event listening by default
  }

  hostDisconnected(): void {
    this.cleanup?.();
  }

  // Play sounds directly - the primary API
  play(sound: SoundType, volume?: number): void {
    triggerSound(sound, volume);
  }
}
