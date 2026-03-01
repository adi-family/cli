import { Logger, trace } from '@adi-family/sdk-plugin';
import type { DataChannelName, RtcState } from './signaling-types.ts';

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

class RtcSessionClient implements RtcSession {
  private readonly log: Logger;
  private readonly pc: RTCPeerConnection;
  private readonly channels = new Map<DataChannelName, RTCDataChannel>();
  private readonly pendingCandidates: RTCIceCandidateInit[] = [];
  private disconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private closed = false;

  constructor(
    private readonly deviceId: string,
    _sessionId: string,
    private readonly handlers: RtcHandlers,
  ) {
    this.log = new Logger('rtc-session', () => ({
      deviceId: this.deviceId.slice(0, 8),
      closed: this.closed,
      channels: this.channels.size,
    }));

    this.pc = new RTCPeerConnection({ iceServers: STUN_SERVERS });
    this.initChannels();
    this.initIceHandlers();
  }

  sendOnChannel(name: DataChannelName, payload: unknown): boolean {
    const ch = this.channels.get(name);
    if (ch?.readyState === 'open') {
      ch.send(JSON.stringify(payload));
      return true;
    }
    return false;
  }

  isChannelOpen(name: DataChannelName): boolean {
    return this.channels.get(name)?.readyState === 'open';
  }

  @trace('closing')
  close(): void {
    if (this.closed) return;
    this.closed = true;
    this.clearDisconnectTimer();

    for (const [name, ch] of this.channels) {
      ch.close();
      this.handlers.onChannelClose(name);
    }
    this.channels.clear();
    this.pc.close();
  }

  @trace('applying answer')
  async applyAnswer(sdp: string): Promise<void> {
    await this.pc.setRemoteDescription({ type: 'answer', sdp });
    for (const c of this.pendingCandidates) {
      await this.pc.addIceCandidate(c);
    }
    this.pendingCandidates.length = 0;
  }

  @trace('adding ice candidate')
  async addIceCandidate(candidate: RTCIceCandidateInit): Promise<void> {
    if (this.pc.remoteDescription) {
      await this.pc.addIceCandidate(candidate);
    } else {
      this.pendingCandidates.push(candidate);
    }
  }

  @trace('creating offer')
  async createOffer(): Promise<string | undefined> {
    this.handlers.onStateChange('connecting');
    const offer = await this.pc.createOffer();
    await this.pc.setLocalDescription(offer);
    return offer.sdp;
  }

  private initChannels(): void {
    for (const name of CHANNELS) {
      const ch = this.pc.createDataChannel(name, { ordered: true });

      ch.onopen = () => {
        this.log.trace({ msg: 'channel open', channel: name });
        this.handlers.onChannelOpen(name);
      };

      ch.onclose = () => {
        this.log.trace({ msg: 'channel closed', channel: name });
        this.handlers.onChannelClose(name);
      };

      ch.onmessage = (event) => {
        this.handlers.onChannelMessage(name, event.data as string);
      };

      this.channels.set(name, ch);
    }
  }

  private initIceHandlers(): void {
    this.pc.onicecandidate = (event) => {
      if (event.candidate) {
        this.handlers.onIceCandidate(event.candidate);
      }
    };

    this.pc.oniceconnectionstatechange = () => {
      if (this.closed) return;
      this.clearDisconnectTimer();

      switch (this.pc.iceConnectionState) {
        case 'connected':
        case 'completed':
          this.log.trace({ msg: 'ICE connected' });
          this.handlers.onStateChange('connected');
          break;
        case 'failed':
          this.log.trace({ msg: 'ICE failed' });
          this.handlers.onStateChange('failed');
          break;
        case 'disconnected':
          this.log.trace({ msg: 'ICE disconnected, grace period', graceMs: ICE_DISCONNECT_GRACE_MS });
          this.disconnectTimer = setTimeout(() => {
            if (this.closed) return;
            if (this.pc.iceConnectionState === 'disconnected' || this.pc.iceConnectionState === 'closed') {
              this.log.trace({ msg: 'ICE still disconnected, marking idle' });
              this.handlers.onStateChange('idle');
            }
          }, ICE_DISCONNECT_GRACE_MS);
          break;
        case 'closed':
          this.handlers.onStateChange('idle');
          break;
      }
    };
  }

  private clearDisconnectTimer(): void {
    if (this.disconnectTimer !== null) {
      clearTimeout(this.disconnectTimer);
      this.disconnectTimer = null;
    }
  }
}

export const createRtcSession = (
  deviceId: string,
  sessionId: string,
  handlers: RtcHandlers,
): RtcSession => new RtcSessionClient(deviceId, sessionId, handlers);
