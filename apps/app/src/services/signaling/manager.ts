import type { EventBus } from '@adi-family/sdk-plugin';
import type {
  SignalingMessage,
  DataChannelName,
  AdiResponse,
  AdiDiscovery,
} from './types.ts';
import { createWebSocket, type WsControl } from './websocket.ts';
import { createRtcSession, type RtcSession } from './webrtc.ts';
import { createAdiChannel, type AdiChannel } from './adi-channel.ts';
import { createConnection, type Connection } from './connection.ts';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface SessionEntry {
  rtc: RtcSession;
  adi: AdiChannel | null;
  deviceId: string;
}

export interface SignalingManager {
  readonly url: string;
  connect(): void;
  disconnect(): void;
  listCocoons(): void;
  spawnCocoon(name?: string): void;
  startSession(deviceId: string): string;
  closeSession(deviceId: string): void;
  sendOnChannel(deviceId: string, channel: DataChannelName, payload: unknown): boolean;
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export const createSignalingManager = (
  url: string,
  connections: Map<string, Connection>,
  bus: EventBus,
  getToken: (authDomain: string) => Promise<string | null>,
): SignalingManager => {
  const sessions = new Map<string, SessionEntry>();
  const deviceToSession = new Map<string, string>();
  let authenticatedUserId: string | null = null;

  // -- WebSocket layer -------------------------------------------------------

  const ws: WsControl = createWebSocket(url, {
    onStateChange: (state) => {
      bus.emit('signaling:state', { url, state });
      if (state === 'disconnected') {
        authenticatedUserId = null;
      }
    },
    onMessage: (msg) => void handleWsMessage(msg),
    onError: (msg) => console.debug('[signaling:manager] ws error:', msg),
  });

  async function handleWsMessage(msg: SignalingMessage): Promise<void> {
    switch (msg.type) {
      case 'hello':
        await handleHello(msg.auth_domain);
        break;

      case 'authenticated':
        authenticatedUserId = msg.user_id;
        listCocoons();
        break;

      case 'my_cocoons':
        bus.emit('signaling:cocoons', { url, cocoons: msg.cocoons });
        break;

      case 'web_rtc_session_started':
        onSessionStarted(msg.session_id, msg.device_id);
        break;

      case 'web_rtc_answer':
        onAnswer(msg.session_id, msg.sdp);
        break;

      case 'web_rtc_ice_candidate':
        onIceCandidate(msg.session_id, {
          candidate: msg.candidate,
          sdpMid: msg.sdp_mid,
          sdpMLineIndex: msg.sdp_mline_index,
        });
        break;

      case 'web_rtc_session_ended':
        onSessionEnded(msg.session_id);
        break;

      case 'web_rtc_error': {
        const entry = sessions.get(msg.session_id);
        if (entry) {
          console.debug(`[signaling:manager] session error: ${msg.message}`);
          bus.emit('signaling:session-state', {
            url,
            deviceId: entry.deviceId,
            state: 'failed',
            sessionId: msg.session_id,
          });
        }
        break;
      }

      case 'web_rtc_data': {
        const entry = sessions.get(msg.session_id);
        if (entry) {
          routeChannelData(entry, msg.channel as DataChannelName, msg.data);
        }
        break;
      }

      case 'spawn_cocoon_result':
        bus.emit('signaling:spawn-result', {
          url,
          requestId: msg.request_id,
          success: msg.success,
          deviceId: msg.device_id,
          error: msg.error,
        });
        if (msg.success) listCocoons();
        break;

      case 'access_denied':
        bus.emit('signaling:auth-error', {
          url,
          reason: msg.reason,
          authKind: msg.auth_kind,
          authDomain: msg.auth_domain,
        });
        // If we thought we were authenticated, reset
        if (authenticatedUserId) {
          authenticatedUserId = null;
        }
        bus.emit('actions:push', {
          id: `auth-error:${url}`,
          plugin: msg.auth_kind ?? 'unknown',
          kind: 'auth-required',
          data: { url, reason: msg.reason, authKind: msg.auth_kind, authDomain: msg.auth_domain },
          priority: 'urgent',
        });
        break;

      case 'error':
        console.debug('[signaling:manager] server error:', msg.message);
        break;

      default:
        break;
    }
  }

  async function handleHello(authDomain: string): Promise<void> {
    const token = await getToken(authDomain);
    if (token) {
      ws.send({ type: 'authenticate', access_token: token });
    } else {
      bus.emit('actions:push', {
        id: `auth-error:${url}`,
        plugin: 'adi.auth',
        kind: 'auth-required',
        data: { url, reason: 'Authentication required', authKind: 'adi.auth', authDomain },
        priority: 'urgent',
      });
    }
  }

  // -- Session lifecycle -----------------------------------------------------

  function onSessionStarted(sessionId: string, _deviceId: string): void {
    const entry = sessions.get(sessionId);
    if (!entry) return;

    void entry.rtc.createOffer().then((sdp) => {
      if (sdp) {
        ws.send({ type: 'web_rtc_offer', session_id: sessionId, sdp });
      }
    });
  }

  function onAnswer(sessionId: string, sdp: string): void {
    const entry = sessions.get(sessionId);
    if (entry) void entry.rtc.applyAnswer(sdp);
  }

  function onIceCandidate(sessionId: string, candidate: RTCIceCandidateInit): void {
    const entry = sessions.get(sessionId);
    if (entry) void entry.rtc.addIceCandidate(candidate);
  }

  function onSessionEnded(sessionId: string): void {
    const entry = sessions.get(sessionId);
    if (!entry) return;
    teardownSession(sessionId, entry);
  }

  function teardownSession(sessionId: string, entry: SessionEntry): void {
    entry.adi?.cancelAll();
    entry.rtc.close();
    sessions.delete(sessionId);
    deviceToSession.delete(entry.deviceId);

    if (connections.has(entry.deviceId)) {
      connections.delete(entry.deviceId);
      bus.emit('connection:removed', { id: entry.deviceId });
    }

    bus.emit('signaling:session-state', {
      url,
      deviceId: entry.deviceId,
      state: 'idle',
      sessionId,
    });
  }

  // -- ADI channel wiring ---------------------------------------------------

  function onAdiChannelOpen(_sessionId: string, entry: SessionEntry): void {
    const adi = createAdiChannel(
      (payload) => entry.rtc.sendOnChannel('adi', payload),
    );
    entry.adi = adi;

    // Discover services and register connection
    void adi.listServices().then((services) => {
      const serviceNames = services.map((s) => s.id);
      const conn = createConnection(entry.deviceId, serviceNames, adi);
      connections.set(entry.deviceId, conn);
      bus.emit('connection:added', { id: entry.deviceId, services: serviceNames });
    }).catch((err) => {
      console.debug('[signaling:manager] service discovery failed:', err);
    });
  }

  // -- Data channel routing --------------------------------------------------

  function routeChannelData(entry: SessionEntry, channel: DataChannelName, raw: string): void {
    if (channel === 'adi' && entry.adi) {
      try {
        entry.adi.handleResponse(JSON.parse(raw) as AdiResponse | AdiDiscovery);
      } catch {
        console.debug('[signaling:manager] failed to parse adi message');
      }
    }
  }

  // -- Public API ------------------------------------------------------------

  const connect = (): void => ws.connect();

  const disconnect = (): void => {
    for (const [sessionId, entry] of sessions) {
      ws.send({ type: 'web_rtc_session_ended', session_id: sessionId, reason: 'disconnect' });
      teardownSession(sessionId, entry);
    }
    ws.disconnect();
  };

  const listCocoons = (): void => {
    ws.send({ type: 'list_my_cocoons' });
  };

  const spawnCocoon = (name?: string): void => {
    const requestId = `spawn-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    ws.send({
      type: 'spawn_cocoon',
      request_id: requestId,
      setup_token: '',
      kind: 'default',
      ...(name ? { name } : {}),
    });
  };

  const startSession = (deviceId: string): string => {
    // Close existing session for this device
    const existingId = deviceToSession.get(deviceId);
    if (existingId) {
      const existing = sessions.get(existingId);
      if (existing) {
        ws.send({ type: 'web_rtc_session_ended', session_id: existingId, reason: 'replaced' });
        teardownSession(existingId, existing);
      }
    }

    const sessionId = `webrtc-${Date.now()}-${Math.random().toString(36).slice(2)}`;

    const rtc = createRtcSession(deviceId, sessionId, {
      onStateChange: (state) => {
        bus.emit('signaling:session-state', { url, deviceId, state, sessionId });
      },
      onIceCandidate: (candidate) => {
        ws.send({
          type: 'web_rtc_ice_candidate',
          session_id: sessionId,
          candidate: candidate.candidate,
          sdp_mid: candidate.sdpMid ?? undefined,
          sdp_mline_index: candidate.sdpMLineIndex ?? undefined,
        });
      },
      onChannelOpen: (name) => {
        console.debug(`[signaling:manager] channel ${name} open for ${deviceId.slice(0, 8)}`);
        const entry = sessions.get(sessionId);
        if (name === 'adi' && entry && !entry.adi) {
          onAdiChannelOpen(sessionId, entry);
        }
      },
      onChannelClose: (name) => {
        console.debug(`[signaling:manager] channel ${name} closed for ${deviceId.slice(0, 8)}`);
      },
      onChannelMessage: (name, data) => {
        const entry = sessions.get(sessionId);
        if (entry) routeChannelData(entry, name, data);
      },
    });

    const entry: SessionEntry = { rtc, adi: null, deviceId };
    sessions.set(sessionId, entry);
    deviceToSession.set(deviceId, sessionId);

    bus.emit('signaling:session-state', { url, deviceId, state: 'signaling', sessionId });

    ws.send({
      type: 'web_rtc_start_session',
      session_id: sessionId,
      device_id: deviceId,
    });

    return sessionId;
  };

  const closeSession = (deviceId: string): void => {
    const sessionId = deviceToSession.get(deviceId);
    if (!sessionId) return;
    const entry = sessions.get(sessionId);
    if (!entry) return;

    ws.send({ type: 'web_rtc_session_ended', session_id: sessionId, reason: 'user_closed' });
    teardownSession(sessionId, entry);
  };

  const sendOnChannel = (deviceId: string, channel: DataChannelName, payload: unknown): boolean => {
    const sessionId = deviceToSession.get(deviceId);
    if (!sessionId) return false;
    const entry = sessions.get(sessionId);
    if (!entry) return false;

    if (entry.rtc.sendOnChannel(channel, payload)) return true;

    // Fallback to signaling relay
    ws.send({
      type: 'web_rtc_data',
      session_id: sessionId,
      channel,
      data: JSON.stringify(payload),
      binary: false,
    });
    return true;
  };

  // -- Listen for auth state changes to re-authenticate ---------------------

  bus.on('auth:state-changed', ({ user }) => {
    if (user && !authenticatedUserId) {
      // User just logged in and we're unauthenticated — reconnect to trigger Hello flow
      ws.disconnect();
      ws.connect();
    }
  });

  return { url, connect, disconnect, listCocoons, spawnCocoon, startSession, closeSession, sendOnChannel };
};
