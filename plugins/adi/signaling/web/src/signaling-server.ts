import { Logger, trace, type EventBus } from '@adi-family/sdk-plugin';
import { ActionsBusKey } from '@adi-family/plugin-actions-feed';
import { AdiAuthBusKey, AdiSignalingBusKey, WsState } from './generated';
import type { RoomInfo } from './generated';
import type { DeviceInfo, SignalingMessage } from './generated/channels';
import { createWebSocket, type WsControl } from './websocket';

export type TokenGetter = (authDomain: string) => Promise<string | null>;

const SOURCE = 'signaling';

export class SignalingServer {
  readonly url: string;

  private readonly log = new Logger('signaling-server', () => ({
    url: this.url,
    state: this.state,
    authenticated: this.authenticatedUserId !== null,
  }));
  private readonly bus: EventBus;
  private readonly ws: WsControl;
  private readonly unsubscribers: (() => void)[] = [];
  private authenticatedUserId: string | null = null;
  private registeredDeviceId: string | null = null;
  private connectedPeers = new Set<string>();
  private knownDevices: DeviceInfo[] = [];
  private knownRooms = new Map<string, RoomInfo>();
  private authenticating = false;
  private disposed = false;
  private state: WsState = WsState.Disconnected;
  constructor(
    url: string,
    bus: EventBus,
    private readonly isStarted: () => boolean,
    private readonly getToken: TokenGetter,
  ) {
    this.url = url;
    this.bus = bus;

    this.ws = createWebSocket(url, {
      onStateChange: (state) => {
        this.state = state;
        bus.emit(AdiSignalingBusKey.State, { url, state }, SOURCE);
        if (state === WsState.Disconnected) {
          this.authenticatedUserId = null;
          this.registeredDeviceId = null;
          this.connectedPeers.clear();
          this.knownDevices = [];
          this.knownRooms.clear();
          this.authenticating = false;
        }
      },
      onMessage: (msg) => void this.handleMessage(msg),
      onError: (msg) => this.log.error({ msg: 'ws error', error: msg }),
    });

    this.unsubscribers.push(
      bus.on(
        AdiSignalingBusKey.AuthAnonymous,
        ({ signalingUrl, authDomain }) => {
          if (signalingUrl !== url) return;
          void this.handleAnonymousAuth(authDomain);
        },
        SOURCE,
      ),
      bus.on(
        AdiAuthBusKey.StateChanged,
        ({ user }) => {
          if (user && !this.authenticatedUserId && !this.authenticating) {
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

  isAuthenticated(): boolean {
    return this.authenticatedUserId !== null;
  }

  getUserId(): string | null {
    return this.authenticatedUserId;
  }

  getDeviceId(): string | null {
    return this.registeredDeviceId;
  }

  getPeers(): ReadonlySet<string> {
    return this.connectedPeers;
  }

  getDevices(): readonly DeviceInfo[] {
    return this.knownDevices;
  }

  @trace('registering device')
  registerDevice(secret: string, version: string, tags?: Record<string, string>, deviceId?: string): void {
    this.ws.send({
      type: 'device_register',
      secret,
      version,
      tags,
      ...(deviceId ? { device_id: deviceId } : {}),
    } as SignalingMessage);
  }

  @trace('deregistering device')
  deregisterDevice(deviceId: string, reason?: string): void {
    this.ws.send({ type: 'device_deregister', device_id: deviceId, reason });
  }

  @trace('updating tags')
  updateTags(tags: Record<string, string>): void {
    this.ws.send({ type: 'device_update_tags', tags });
  }

  @trace('updating device')
  updateDevice(tags?: Record<string, string>, deviceConfig?: unknown): void {
    this.ws.send({ type: 'device_update_device', tags, device_config: deviceConfig });
  }

  @trace('creating pairing code')
  createPairingCode(): void {
    this.ws.send({ type: 'pairing_create_code' });
  }

  @trace('using pairing code')
  usePairingCode(code: string): void {
    this.ws.send({ type: 'pairing_use_code', code });
  }

  @trace('sending sync data')
  sendSyncData(payload: unknown): void {
    this.ws.send({ type: 'sync_data', payload });
  }

  getRooms(): ReadonlyMap<string, RoomInfo> {
    return this.knownRooms;
  }

  @trace('creating room')
  createRoom(roomId?: string): void {
    this.ws.send({ type: 'room_create', ...(roomId ? { room_id: roomId } : {}) } as SignalingMessage);
  }

  @trace('deleting room')
  deleteRoom(roomId: string): void {
    this.ws.send({ type: 'room_delete', room_id: roomId });
  }

  @trace('adding actor to room')
  addRoomActor(roomId: string, deviceId: string): void {
    this.ws.send({ type: 'room_add_actor', room_id: roomId, device_id: deviceId });
  }

  @trace('removing actor from room')
  removeRoomActor(roomId: string, deviceId: string): void {
    this.ws.send({ type: 'room_remove_actor', room_id: roomId, device_id: deviceId });
  }

  @trace('granting room access')
  grantRoomAccess(roomId: string, userId: string): void {
    this.ws.send({ type: 'room_grant_access', room_id: roomId, user_id: userId });
  }

  @trace('revoking room access')
  revokeRoomAccess(roomId: string, userId: string): void {
    this.ws.send({ type: 'room_revoke_access', room_id: roomId, user_id: userId });
  }

  @trace('listing rooms')
  listRooms(): void {
    this.ws.send({ type: 'room_list' });
  }

  @trace('getting room')
  getRoom(roomId: string): void {
    this.ws.send({ type: 'room_get', room_id: roomId });
  }

  @trace('sending room message')
  sendRoomMessage(roomId: string, payload: unknown, to?: string): void {
    this.ws.send({ type: 'room_send', room_id: roomId, payload, ...(to ? { to } : {}) } as SignalingMessage);
  }

  @trace('connecting')
  connect(): void {
    if (this.disposed || !this.isStarted()) return;
    this.ws.connect();
  }

  @trace('disconnecting')
  disconnect(): void {
    this.disposed = true;
    this.ws.disconnect();
    this.unsubscribers.forEach((fn) => fn());
    this.unsubscribers.length = 0;
  }

  @trace('querying devices')
  queryDevices(tagFilter: Record<string, string>): void {
    this.ws.send({ type: 'device_query_devices', tag_filter: tagFilter });
  }

  send(msg: SignalingMessage): void {
    this.ws.send(msg);
  }

  // -- Message handling -------------------------------------------------------

  @trace('handling message')
  private async handleMessage(msg: SignalingMessage): Promise<void> {
    switch (msg.type) {
      case 'auth_hello':
        await this.handleHello(
          msg.auth_kind,
          msg.auth_domain,
          msg.auth_requirement,
          msg.auth_options,
        );
        break;

      case 'auth_authenticate_response':
        this.authenticating = false;
        this.authenticatedUserId = msg.user_id;
        this.bus.emit(
          AdiSignalingBusKey.AuthOk,
          { url: this.url, userId: msg.user_id },
          SOURCE,
        );
        break;

      case 'auth_hello_authed':
        this.bus.emit(
          AdiSignalingBusKey.ConnectionInfo,
          { url: this.url, connectionInfo: msg.connection_info },
          SOURCE,
        );
        this.knownDevices = msg.devices;
        this.bus.emit(
          AdiSignalingBusKey.Devices,
          { url: this.url, devices: msg.devices },
          SOURCE,
        );
        break;

      case 'device_device_list_updated':
        this.knownDevices = msg.devices;
        this.bus.emit(
          AdiSignalingBusKey.Devices,
          { url: this.url, devices: msg.devices },
          SOURCE,
        );
        break;

      case 'device_register_response':
        this.registeredDeviceId = msg.device_id;
        this.bus.emit(
          AdiSignalingBusKey.DeviceRegistered,
          { url: this.url, deviceId: msg.device_id, tags: msg.tags },
          SOURCE,
        );
        break;

      case 'device_deregister_response':
        if (msg.device_id === this.registeredDeviceId) {
          this.registeredDeviceId = null;
        }
        this.bus.emit(
          AdiSignalingBusKey.DeviceDeregistered,
          { url: this.url, deviceId: msg.device_id },
          SOURCE,
        );
        break;

      case 'device_update_tags_response':
        this.bus.emit(
          AdiSignalingBusKey.TagsUpdated,
          { url: this.url, deviceId: msg.device_id, tags: msg.tags },
          SOURCE,
        );
        break;

      case 'device_update_device_response':
        this.bus.emit(
          AdiSignalingBusKey.DeviceUpdated,
          { url: this.url, deviceId: msg.device_id, tags: msg.tags, deviceConfig: msg.device_config },
          SOURCE,
        );
        break;

      case 'device_query_devices_response':
        this.bus.emit(
          AdiSignalingBusKey.Devices,
          { url: this.url, devices: msg.devices },
          SOURCE,
        );
        break;

      case 'device_peer_connected':
        this.connectedPeers.add(msg.peer_id);
        this.bus.emit(
          AdiSignalingBusKey.PeerConnected,
          { url: this.url, peerId: msg.peer_id },
          SOURCE,
        );
        break;

      case 'device_peer_disconnected':
        this.connectedPeers.delete(msg.peer_id);
        this.bus.emit(
          AdiSignalingBusKey.PeerDisconnected,
          { url: this.url, peerId: msg.peer_id },
          SOURCE,
        );
        break;

      case 'pairing_create_code_response':
        this.bus.emit(
          AdiSignalingBusKey.PairingCode,
          { url: this.url, code: msg.code },
          SOURCE,
        );
        break;

      case 'pairing_use_code_response':
        this.connectedPeers.add(msg.peer_id);
        this.bus.emit(
          AdiSignalingBusKey.PairingConnected,
          { url: this.url, peerId: msg.peer_id },
          SOURCE,
        );
        break;

      case 'pairing_failed':
        this.bus.emit(
          AdiSignalingBusKey.PairingFailed,
          { url: this.url, reason: msg.reason },
          SOURCE,
        );
        break;

      case 'sync_data':
        this.bus.emit(
          AdiSignalingBusKey.SyncData,
          { url: this.url, payload: msg.payload },
          SOURCE,
        );
        break;

      case 'room_create_response':
        this.listRooms();
        break;

      case 'room_delete_response':
        this.knownRooms.delete(msg.room_id);
        break;

      case 'room_add_actor_response':
      case 'room_remove_actor_response':
      case 'room_grant_access_response':
      case 'room_revoke_access_response':
        break;

      case 'room_list_response':
        this.knownRooms.clear();
        for (const room of msg.rooms) {
          this.knownRooms.set(room.room_id, room);
        }
        break;

      case 'room_get_response':
        this.knownRooms.set(msg.room_id, {
          room_id: msg.room_id,
          owner_user_id: msg.owner_user_id,
          granted_users: msg.granted_users,
          actors: msg.actors,
        });
        break;

      case 'room_updated':
        this.knownRooms.set(msg.room.room_id, msg.room);
        this.bus.emit(
          AdiSignalingBusKey.RoomUpdated,
          { url: this.url, room: msg.room },
          SOURCE,
        );
        break;

      case 'room_actor_joined':
        this.bus.emit(
          AdiSignalingBusKey.RoomActorJoined,
          { url: this.url, roomId: msg.room_id, deviceId: msg.device_id },
          SOURCE,
        );
        break;

      case 'room_actor_left':
        this.bus.emit(
          AdiSignalingBusKey.RoomActorLeft,
          { url: this.url, roomId: msg.room_id, deviceId: msg.device_id },
          SOURCE,
        );
        break;

      case 'room_send':
        this.bus.emit(
          AdiSignalingBusKey.RoomMessage,
          { url: this.url, roomId: msg.room_id, from: '', payload: msg.payload },
          SOURCE,
        );
        break;

      case 'system_error':
        this.log.error({ msg: 'server error', error: msg.message });
        break;

      default:
        break;
    }
  }

  @trace('handling hello')
  private async handleHello(
    authKind: string,
    authDomain: string,
    authRequirement: string,
    authOptions: string[],
  ): Promise<void> {
    this.authenticating = true;

    const token = await this.getToken(authDomain);
    if (token) {
      this.ws.send({ type: 'auth_authenticate', access_token: token });
      return;
    }

    if (authRequirement === 'required') {
      this.bus.emit(ActionsBusKey.Push, {
        id: `signaling:auth:${this.url}`,
        plugin: 'adi.auth',
        kind: 'auth-required',
        data: {
          authKind,
          authDomain,
          authRequirement,
          authOptions,
          signalingUrl: this.url,
          reason: `Signaling server requires ${authKind} authentication`,
        },
        priority: 'urgent',
      }, SOURCE);
    }

    this.authenticating = false;
    this.bus.emit(
      AdiSignalingBusKey.AuthError,
      { url: this.url, reason: `No token for ${authDomain}` },
      SOURCE,
    );
  }

  @trace('handling anonymous auth')
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
        AdiAuthBusKey.SessionSave,
        {
          accessToken: token,
          email: '',
          expiresAt: Date.now() + expiresIn * 1000,
          authUrl: authDomain,
        },
        SOURCE,
      );

      this.authenticating = true;
      this.ws.send({ type: 'auth_authenticate', access_token: token });
    } catch (err) {
      this.log.warn({
        msg: 'anonymous auth failed',
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }
}
