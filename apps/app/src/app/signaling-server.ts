import { Logger, type EventBus } from '@adi-family/sdk-plugin';
import type {
  SignalingMessage,
  DataChannelName,
  AdiResponse,
  AdiDiscovery,
  WsState,
  CocoonInfo,
  HiveInfo,
} from '../services/signaling/types.ts';
import { createWebSocket, type WsControl } from '../services/signaling/websocket.ts';
import { createRtcSession, type RtcSession } from '../services/signaling/webrtc.ts';
import { createAdiChannel, type AdiChannel } from '../services/signaling/adi-channel.ts';
import { createConnection, type Connection } from '../services/signaling/connection.ts';

interface SessionEntry {
  rtc: RtcSession;
  adi: AdiChannel | null;
  deviceId: string;
}

const SOURCE = 'signaling';

export class SignalingServer {
  readonly url: string;

  private readonly log = new Logger('signaling-server');
  private readonly bus: EventBus;
  private readonly connections: Map<string, Connection>;
  private readonly ws: WsControl;
  private readonly sessions = new Map<string, SessionEntry>();
  private readonly deviceToSession = new Map<string, string>();
  private readonly unsubscribers: (() => void)[] = [];
  private authenticatedUserId: string | null = null;
  private lastAuthOptions: string[] = [];
  private disposed = false;
  private state: WsState = 'disconnected';
  private cocoons: CocoonInfo[] = [];
  private hives: HiveInfo[] = [];
  private pendingSetupToken: {
    resolve: (token: string) => void;
    reject: (err: Error) => void;
  } | null = null;

  constructor(
    url: string,
    connections: Map<string, Connection>,
    bus: EventBus,
  ) {
    this.url = url;
    this.connections = connections;
    this.bus = bus;

    this.ws = createWebSocket(url, {
      onStateChange: (state) => {
        bus.emit('signaling:state', { url, state }, SOURCE);
        if (state === 'disconnected') {
          this.authenticatedUserId = null;
        }
      },
      onMessage: (msg) => void this.handleWsMessage(msg),
      onError: (msg) => this.log.error({ msg: 'ws error', error: msg }),
    });

    this.unsubscribers.push(
      bus.on(
        'signaling:state',
        ({ url: u, state }) => {
          if (u !== url) return;
          this.state = state;
        },
        SOURCE,
      ),
      bus.on(
        'signaling:cocoons',
        ({ url: u, cocoons }) => {
          if (u !== url) return;
          this.cocoons = cocoons;
        },
        SOURCE,
      ),
      bus.on(
        'signaling:hives',
        ({ url: u, hives }) => {
          if (u !== url) return;
          this.hives = hives;
        },
        SOURCE,
      ),
      bus.on(
        'signaling:auth-anonymous',
        ({ signalingUrl, authDomain }) => {
          if (signalingUrl !== url) return;
          void this.handleAnonymousAuth(authDomain);
        },
        SOURCE,
      ),
      bus.on(
        'auth:state-changed',
        ({ user }) => {
          if (user && !this.authenticatedUserId) {
            this.ws.disconnect();
            this.ws.connect();
          }
        },
        SOURCE,
      ),
    );
  }

  getState(): WsState {
    return this.state;
  }

  getCocoons(): readonly CocoonInfo[] {
    return this.cocoons;
  }

  getHives(): readonly HiveInfo[] {
    return this.hives;
  }

  connect(): void {
    if (this.disposed) return;
    this.ws.connect();
  }

  disconnect(): void {
    this.disposed = true;
    for (const [sessionId, entry] of this.sessions) {
      this.ws.send({
        type: 'web_rtc_session_ended',
        session_id: sessionId,
        reason: 'disconnect',
      });
      this.teardownSession(sessionId, entry);
    }
    this.ws.disconnect();
    this.unsubscribers.forEach((fn) => fn());
    this.unsubscribers.length = 0;
  }

  listCocoons(): void {
    this.ws.send({ type: 'list_my_cocoons' });
  }

  listHives(): void {
    this.ws.send({ type: 'list_hives' });
  }

  requestSetupToken(): Promise<string> {
    return new Promise((resolve, reject) => {
      this.pendingSetupToken = { resolve, reject };
      this.ws.send({ type: 'request_setup_token' });
      setTimeout(() => {
        if (this.pendingSetupToken) {
          this.pendingSetupToken.reject(
            new Error('Setup token request timed out'),
          );
          this.pendingSetupToken = null;
        }
      }, 10_000);
    });
  }

  spawnCocoon(name?: string, kind?: string): void {
    const requestId = `spawn-${Date.now()}-${Math.random().toString(36).slice(2)}`;

    this.requestSetupToken()
      .then((token) => {
        this.ws.send({
          type: 'spawn_cocoon',
          request_id: requestId,
          setup_token: token,
          kind: kind ?? 'ubuntu',
          ...(name ? { name } : {}),
        });
      })
      .catch((err) => {
        this.bus.emit(
          'signaling:spawn-result',
          {
            url: this.url,
            requestId,
            success: false,
            error: `Failed to get setup token: ${err instanceof Error ? err.message : String(err)}`,
          },
          SOURCE,
        );
      });
  }

  startSession(deviceId: string): string {
    const existingId = this.deviceToSession.get(deviceId);
    if (existingId) {
      const existing = this.sessions.get(existingId);
      if (existing) {
        this.ws.send({
          type: 'web_rtc_session_ended',
          session_id: existingId,
          reason: 'replaced',
        });
        this.teardownSession(existingId, existing);
      }
    }

    const sessionId = `webrtc-${Date.now()}-${Math.random().toString(36).slice(2)}`;

    const rtc = createRtcSession(deviceId, sessionId, {
      onStateChange: (state) => {
        this.bus.emit(
          'signaling:session-state',
          { url: this.url, deviceId, state, sessionId },
          SOURCE,
        );
      },
      onIceCandidate: (candidate) => {
        this.ws.send({
          type: 'web_rtc_ice_candidate',
          session_id: sessionId,
          candidate: candidate.candidate,
          sdp_mid: candidate.sdpMid ?? undefined,
          sdp_mline_index: candidate.sdpMLineIndex ?? undefined,
        });
      },
      onChannelOpen: (channelName) => {
        this.log.trace({
          msg: 'channel open',
          channel: channelName,
          deviceId: deviceId.slice(0, 8),
        });
        const entry = this.sessions.get(sessionId);
        if (channelName === 'adi' && entry && !entry.adi) {
          this.onAdiChannelOpen(entry);
        }
      },
      onChannelClose: (channelName) => {
        this.log.trace({
          msg: 'channel closed',
          channel: channelName,
          deviceId: deviceId.slice(0, 8),
        });
      },
      onChannelMessage: (channelName, data) => {
        const entry = this.sessions.get(sessionId);
        if (entry) this.routeChannelData(entry, channelName, data);
      },
    });

    const entry: SessionEntry = { rtc, adi: null, deviceId };
    this.sessions.set(sessionId, entry);
    this.deviceToSession.set(deviceId, sessionId);

    this.bus.emit(
      'signaling:session-state',
      { url: this.url, deviceId, state: 'signaling', sessionId },
      SOURCE,
    );

    this.ws.send({
      type: 'web_rtc_start_session',
      session_id: sessionId,
      device_id: deviceId,
    });

    return sessionId;
  }

  closeSession(deviceId: string): void {
    const sessionId = this.deviceToSession.get(deviceId);
    if (!sessionId) return;
    const entry = this.sessions.get(sessionId);
    if (!entry) return;

    this.ws.send({
      type: 'web_rtc_session_ended',
      session_id: sessionId,
      reason: 'user_closed',
    });
    this.teardownSession(sessionId, entry);
  }

  sendOnChannel(
    deviceId: string,
    channel: DataChannelName,
    payload: unknown,
  ): boolean {
    const sessionId = this.deviceToSession.get(deviceId);
    if (!sessionId) return false;
    const entry = this.sessions.get(sessionId);
    if (!entry) return false;

    if (entry.rtc.sendOnChannel(channel, payload)) return true;

    this.ws.send({
      type: 'web_rtc_data',
      session_id: sessionId,
      channel,
      data: JSON.stringify(payload),
      binary: false,
    });
    return true;
  }

  // -- WebSocket message handling ---------------------------------------------

  private async handleWsMessage(msg: SignalingMessage): Promise<void> {
    switch (msg.type) {
      case 'hello':
        await this.handleHello(
          msg.auth_kind,
          msg.auth_domain,
          msg.auth_requirement,
          msg.auth_options,
        );
        break;

      case 'authenticated':
        this.authenticatedUserId = msg.user_id;
        this.bus.emit('signaling:auth-ok', { url: this.url }, SOURCE);
        this.bus.emit(
          'actions:dismiss',
          { id: `auth-required:${this.url}` },
          SOURCE,
        );
        this.bus.emit(
          'actions:dismiss',
          { id: `auth-error:${this.url}` },
          SOURCE,
        );
        break;

      case 'hello_authed':
        this.bus.emit(
          'signaling:connection-info',
          { url: this.url, connectionInfo: msg.connection_info },
          SOURCE,
        );
        this.listCocoons();
        this.listHives();
        break;

      case 'my_cocoons':
        this.bus.emit(
          'signaling:cocoons',
          { url: this.url, cocoons: msg.cocoons },
          SOURCE,
        );
        break;

      case 'hives_list':
        this.bus.emit(
          'signaling:hives',
          { url: this.url, hives: msg.hives },
          SOURCE,
        );
        break;

      case 'web_rtc_session_started':
        this.onSessionStarted(msg.session_id);
        break;

      case 'web_rtc_answer':
        this.onAnswer(msg.session_id, msg.sdp);
        break;

      case 'web_rtc_ice_candidate':
        this.onIceCandidate(msg.session_id, {
          candidate: msg.candidate,
          sdpMid: msg.sdp_mid,
          sdpMLineIndex: msg.sdp_mline_index,
        });
        break;

      case 'web_rtc_session_ended':
        this.onSessionEnded(msg.session_id);
        break;

      case 'web_rtc_error': {
        const entry = this.sessions.get(msg.session_id);
        if (entry) {
          this.log.error({
            msg: 'session error',
            sessionId: msg.session_id,
            error: msg.message,
          });
          this.bus.emit(
            'signaling:session-state',
            {
              url: this.url,
              deviceId: entry.deviceId,
              state: 'failed',
              sessionId: msg.session_id,
            },
            SOURCE,
          );
        }
        break;
      }

      case 'web_rtc_data': {
        const entry = this.sessions.get(msg.session_id);
        if (entry) {
          this.routeChannelData(
            entry,
            msg.channel as DataChannelName,
            msg.data,
          );
        }
        break;
      }

      case 'spawn_cocoon_result':
        this.bus.emit(
          'signaling:spawn-result',
          {
            url: this.url,
            requestId: msg.request_id,
            success: msg.success,
            deviceId: msg.device_id,
            error: msg.error,
          },
          SOURCE,
        );
        if (msg.success) this.listCocoons();
        break;

      case 'access_denied':
        this.bus.emit(
          'signaling:auth-error',
          {
            url: this.url,
            reason: msg.reason,
            authKind: msg.auth_kind,
            authDomain: msg.auth_domain,
          },
          SOURCE,
        );
        if (this.authenticatedUserId) {
          this.authenticatedUserId = null;
        }
        this.bus.emit(
          'actions:push',
          {
            id: `auth-error:${this.url}`,
            plugin: msg.auth_kind ?? 'unknown',
            kind: 'auth-required',
            data: {
              url: this.url,
              reason: msg.reason,
              authKind: msg.auth_kind,
              authDomain: msg.auth_domain,
              authOptions: this.lastAuthOptions,
            },
            priority: 'urgent',
          },
          SOURCE,
        );
        break;

      case 'setup_token':
        if (this.pendingSetupToken) {
          this.pendingSetupToken.resolve(msg.token);
          this.pendingSetupToken = null;
        }
        break;

      case 'error':
        this.log.error({ msg: 'server error', error: msg.message });
        if (this.pendingSetupToken) {
          this.pendingSetupToken.reject(new Error(msg.message));
          this.pendingSetupToken = null;
        }
        break;

      default:
        break;
    }
  }

  private async handleHello(
    authKind: string,
    authDomain: string,
    authRequirement: string,
    authOptions: string[],
  ): Promise<void> {
    this.lastAuthOptions = authOptions;

    const { token } = await this.bus
      .send(
        'auth:get-token',
        { authDomain, sourceUrl: this.url },
        SOURCE,
      )
      .wait();

    if (token) {
      this.ws.send({ type: 'authenticate', access_token: token });
      return;
    }

    this.bus.emit(
      'actions:push',
      {
        id: `auth-required:${this.url}`,
        plugin: authKind,
        kind: 'auth-required',
        data: {
          url: this.url,
          authKind,
          authDomain,
          authRequirement,
          authOptions,
        },
        priority: 'urgent',
      },
      SOURCE,
    );
  }

  private async handleAnonymousAuth(authDomain: string): Promise<void> {
    try {
      const res = await fetch(`${authDomain}/anonymous`, { method: 'POST' });
      if (!res.ok) return;
      const data = (await res.json()) as {
        accessToken?: string;
        access_token?: string;
        expiresIn?: number;
        expires_in?: number;
      };
      const token = data.accessToken ?? data.access_token;
      if (!token) return;

      const expiresIn = data.expiresIn ?? data.expires_in ?? 7 * 24 * 3600;
      this.bus.emit(
        'auth:session-save',
        {
          accessToken: token,
          email: '',
          expiresAt: Date.now() + expiresIn * 1000,
          authUrl: authDomain,
        },
        SOURCE,
      );

      this.ws.send({ type: 'authenticate', access_token: token });
    } catch (err) {
      this.log.warn({
        msg: 'anonymous auth failed',
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  // -- Session lifecycle ------------------------------------------------------

  private onSessionStarted(sessionId: string): void {
    const entry = this.sessions.get(sessionId);
    if (!entry) return;

    void entry.rtc.createOffer().then((sdp) => {
      if (sdp) {
        this.ws.send({ type: 'web_rtc_offer', session_id: sessionId, sdp });
      }
    });
  }

  private onAnswer(sessionId: string, sdp: string): void {
    const entry = this.sessions.get(sessionId);
    if (entry) void entry.rtc.applyAnswer(sdp);
  }

  private onIceCandidate(
    sessionId: string,
    candidate: RTCIceCandidateInit,
  ): void {
    const entry = this.sessions.get(sessionId);
    if (entry) void entry.rtc.addIceCandidate(candidate);
  }

  private onSessionEnded(sessionId: string): void {
    const entry = this.sessions.get(sessionId);
    if (!entry) return;
    this.teardownSession(sessionId, entry);
  }

  private teardownSession(sessionId: string, entry: SessionEntry): void {
    entry.adi?.cancelAll();
    entry.rtc.close();
    this.sessions.delete(sessionId);
    this.deviceToSession.delete(entry.deviceId);

    if (this.connections.has(entry.deviceId)) {
      this.connections.delete(entry.deviceId);
      this.bus.emit(
        'connection:removed',
        { id: entry.deviceId },
        SOURCE,
      );
    }

    this.bus.emit(
      'signaling:session-state',
      {
        url: this.url,
        deviceId: entry.deviceId,
        state: 'idle',
        sessionId,
      },
      SOURCE,
    );
  }

  // -- ADI channel wiring ----------------------------------------------------

  private onAdiChannelOpen(entry: SessionEntry): void {
    const adi = createAdiChannel((payload) =>
      entry.rtc.sendOnChannel('adi', payload),
    );
    entry.adi = adi;

    void adi
      .listServices()
      .then((services) => {
        const serviceNames = services.map((s) => s.id);
        const conn = createConnection(entry.deviceId, serviceNames, adi);
        this.connections.set(entry.deviceId, conn);
        this.bus.emit(
          'connection:added',
          { id: entry.deviceId, services: serviceNames },
          SOURCE,
        );
      })
      .catch((err) => {
        this.log.warn({
          msg: 'service discovery failed',
          error: err instanceof Error ? err.message : String(err),
        });
      });
  }

  // -- Data channel routing ---------------------------------------------------

  private routeChannelData(
    entry: SessionEntry,
    channel: DataChannelName,
    raw: string,
  ): void {
    if (channel === 'adi' && entry.adi) {
      try {
        entry.adi.handleResponse(
          JSON.parse(raw) as AdiResponse | AdiDiscovery,
        );
      } catch {
        this.log.warn({ msg: 'failed to parse adi message' });
      }
    }
  }
}

declare module '@adi-family/sdk-plugin' {
  interface EventRegistry {
    'signaling:auth-ok': { url: string };
    'auth:session-save': {
      accessToken: string;
      email: string;
      expiresAt: number;
      authUrl: string;
    };
    'auth:state-changed': { user: unknown };
    'actions:push': {
      id: string;
      plugin: string;
      kind: string;
      data: Record<string, unknown>;
      priority?: 'low' | 'normal' | 'urgent';
    };
    'actions:dismiss': { id: string };
  }
}
