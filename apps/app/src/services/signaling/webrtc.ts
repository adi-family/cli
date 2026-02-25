import type { DataChannelName, RtcState } from './types.ts';

export interface RtcHandlers {
  onStateChange(state: RtcState): void;
  onIceCandidate(candidate: RTCIceCandidate): void;
  onChannelOpen(name: DataChannelName): void;
  onChannelClose(name: DataChannelName): void;
  onChannelMessage(name: DataChannelName, data: string): void;
}

export interface RtcSession {
  sendOnChannel(name: DataChannelName, payload: unknown): boolean;
  isChannelOpen(name: DataChannelName): boolean;
  close(): void;
  applyAnswer(sdp: string): Promise<void>;
  addIceCandidate(candidate: RTCIceCandidateInit): Promise<void>;
  createOffer(): Promise<string | undefined>;
}

const STUN_SERVERS: RTCIceServer[] = [
  { urls: 'stun:stun.l.google.com:19302' },
  { urls: 'stun:stun1.l.google.com:19302' },
];

const CHANNELS: DataChannelName[] = ['terminal', 'silk', 'file', 'pty', 'adi'];
const ICE_DISCONNECT_GRACE_MS = 5000;

export const createRtcSession = (
  deviceId: string,
  _sessionId: string,
  handlers: RtcHandlers,
): RtcSession => {
  const pc = new RTCPeerConnection({ iceServers: STUN_SERVERS });
  const channels = new Map<DataChannelName, RTCDataChannel>();
  const pendingCandidates: RTCIceCandidateInit[] = [];
  let disconnectTimer: ReturnType<typeof setTimeout> | null = null;
  let closed = false;

  const tag = `[signaling:rtc ${deviceId.slice(0, 8)}]`;

  const clearDisconnectTimer = (): void => {
    if (disconnectTimer !== null) {
      clearTimeout(disconnectTimer);
      disconnectTimer = null;
    }
  };

  // Create all data channels
  for (const name of CHANNELS) {
    const ch = pc.createDataChannel(name, { ordered: true });

    ch.onopen = () => {
      console.debug(`${tag} channel ${name} open`);
      handlers.onChannelOpen(name);
    };

    ch.onclose = () => {
      console.debug(`${tag} channel ${name} closed`);
      handlers.onChannelClose(name);
    };

    ch.onmessage = (event) => {
      handlers.onChannelMessage(name, event.data as string);
    };

    channels.set(name, ch);
  }

  // ICE candidate gathering
  pc.onicecandidate = (event) => {
    if (event.candidate) {
      handlers.onIceCandidate(event.candidate);
    }
  };

  // ICE connection state tracking
  pc.oniceconnectionstatechange = () => {
    if (closed) return;
    clearDisconnectTimer();

    switch (pc.iceConnectionState) {
      case 'connected':
      case 'completed':
        console.debug(`${tag} ICE connected`);
        handlers.onStateChange('connected');
        break;
      case 'failed':
        console.debug(`${tag} ICE failed`);
        handlers.onStateChange('failed');
        break;
      case 'disconnected':
        console.debug(`${tag} ICE disconnected, grace period ${ICE_DISCONNECT_GRACE_MS}ms`);
        disconnectTimer = setTimeout(() => {
          if (closed) return;
          if (pc.iceConnectionState === 'disconnected' || pc.iceConnectionState === 'closed') {
            console.debug(`${tag} ICE still disconnected, marking idle`);
            handlers.onStateChange('idle');
          }
        }, ICE_DISCONNECT_GRACE_MS);
        break;
      case 'closed':
        handlers.onStateChange('idle');
        break;
    }
  };

  const sendOnChannel = (name: DataChannelName, payload: unknown): boolean => {
    const ch = channels.get(name);
    if (ch?.readyState === 'open') {
      ch.send(JSON.stringify(payload));
      return true;
    }
    return false;
  };

  const isChannelOpen = (name: DataChannelName): boolean =>
    channels.get(name)?.readyState === 'open';

  const close = (): void => {
    if (closed) return;
    closed = true;
    clearDisconnectTimer();

    for (const [name, ch] of channels) {
      ch.close();
      handlers.onChannelClose(name);
    }
    channels.clear();
    pc.close();
    console.debug(`${tag} session closed`);
  };

  const applyAnswer = async (sdp: string): Promise<void> => {
    await pc.setRemoteDescription({ type: 'answer', sdp });
    // Flush buffered candidates
    for (const c of pendingCandidates) {
      await pc.addIceCandidate(c);
    }
    pendingCandidates.length = 0;
  };

  const addIceCandidate = async (candidate: RTCIceCandidateInit): Promise<void> => {
    if (pc.remoteDescription) {
      await pc.addIceCandidate(candidate);
    } else {
      pendingCandidates.push(candidate);
    }
  };

  const createOffer = async (): Promise<string | undefined> => {
    handlers.onStateChange('connecting');
    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);
    return offer.sdp;
  };

  return { sendOnChannel, isChannelOpen, close, applyAnswer, addIceCandidate, createOffer };
};
