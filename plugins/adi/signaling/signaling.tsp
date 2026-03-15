// ADI Signaling Protocol — WebSocket message definitions
// Source of truth for code generation via lib-typespec-api protocol codegen.
//
// Regenerate: build.rs runs automatically on `cargo build`

// ── Supporting Types ────────────────────────────────────────

enum WsState {
    disconnected: "disconnected",
    connecting: "connecting",
    connected: "connected",
    error: "error",
}

enum AuthRequirement {
    required: "required",
    optional: "optional",
}

enum AuthOption {
    verified: "verified",
    anonymous: "anonymous",
}

model IceServer {
    urls: string[];
    username?: string;
    credential?: string;
}

model ConnectionInfo {
    manual_allowed: boolean;
    ice_servers?: IceServer[];
}

model DeviceInfo {
    device_id: string;
    tags: Record<string>;
    online: boolean;
    device_type?: string;
    device_config?: unknown;
}

// ── Auth Channel ────────────────────────────────────────────

@channel("auth")
interface Auth {
    @serverPush
    hello(
        auth_kind: string,
        auth_domain: string,
        auth_requirement: AuthRequirement,
        auth_options: AuthOption[],
    ): void;

    @request
    authenticate(access_token: string): {
        user_id: string;
    };

    @serverPush
    helloAuthed(
        user_id: string,
        connection_info: ConnectionInfo,
        devices: DeviceInfo[],
    ): void;
}

// ── Device Channel ──────────────────────────────────────────

@channel("device")
interface Device {
    @request
    register(
        secret: string,
        device_id?: string,
        version: string,
        tags?: Record<string>,
        device_type?: string,
        device_config?: unknown,
    ): {
        device_id: string;
        tags?: Record<string>;
    };

    @request
    deregister(device_id: string, reason?: string): {
        device_id: string;
    };

    @event
    peerConnected(peer_id: string): void;

    @event
    peerDisconnected(peer_id: string): void;

    @request
    updateTags(tags: Record<string>): {
        device_id: string;
        tags: Record<string>;
    };

    @request
    updateDevice(
        tags?: Record<string>,
        device_config?: unknown,
    ): {
        device_id: string;
        tags: Record<string>;
        device_config?: unknown;
    };

    @request
    queryDevices(tag_filter: Record<string>): {
        devices: DeviceInfo[];
    };

    @serverPush
    deviceListUpdated(devices: DeviceInfo[]): void;
}

// ── Pairing Channel ─────────────────────────────────────────

@channel("pairing")
interface Pairing {
    @request
    createCode(): {
        code: string;
    };

    @request
    useCode(code: string): {
        peer_id: string;
    };

    @event
    failed(reason: string): void;
}

// ── Sync Channel ────────────────────────────────────────────

@channel("sync")
interface Sync {
    @relay
    data(payload: unknown): void;
}

// ── Hive Channel ───────────────────────────────────────

model CocoonKind {
    id: string;
    runner_type: string;
    runner_config: unknown;
    image: string;
}

@channel("hive")
interface Hive {
    @request
    register(
        hive_id: string,
        version: string,
        cocoon_kinds: CocoonKind[],
        hive_id_signature: string,
    ): {
        hive_id: string;
    };

    @serverPush
    spawnCocoon(
        request_id: string,
        setup_token: string,
        name?: string,
        kind: string,
    ): void;

    @serverPush
    terminateCocoon(
        request_id: string,
        container_id: string,
    ): void;

    @event
    spawnCocoonResult(
        request_id: string,
        success: boolean,
        device_id?: string,
        container_id?: string,
        error?: string,
    ): void;

    @event
    terminateCocoonResult(
        request_id: string,
        success: boolean,
        error?: string,
    ): void;
}

// ── Room Types ─────────────────────────────────────────────

model RoomInfo {
    room_id: string;
    owner_user_id: string;
    granted_users: string[];
    actors: DeviceInfo[];
}

// ── Room Channel ───────────────────────────────────────────

@channel("room")
interface Room {
    @request
    create(room_id?: string): {
        room_id: string;
    };

    @request
    delete(room_id: string): {
        room_id: string;
    };

    @request
    addActor(room_id: string, device_id: string): {
        room_id: string;
        device_id: string;
    };

    @request
    removeActor(room_id: string, device_id: string): {
        room_id: string;
        device_id: string;
    };

    @request
    grantAccess(room_id: string, user_id: string): {
        room_id: string;
        user_id: string;
    };

    @request
    revokeAccess(room_id: string, user_id: string): {
        room_id: string;
        user_id: string;
    };

    @request
    list(): {
        rooms: RoomInfo[];
    };

    @request
    get(room_id: string): RoomInfo;

    @relay
    send(room_id: string, to?: string, payload: unknown): void;

    @event
    actorJoined(room_id: string, device_id: string): void;

    @event
    actorLeft(room_id: string, device_id: string): void;

    @serverPush
    updated(room: RoomInfo): void;
}

// ── System Channel ──────────────────────────────────────────

@channel("system")
interface System {
    @event
    error(message: string): void;
}

// ── EventBus: Signaling ────────────────────────────────────

@bus("adi.signaling")
interface SignalingBus {
    @event state(url: string, state: WsState): void;
    @event authOk(url: string, userId: string): void;
    @event authError(url: string, reason: string): void;
    @event connectionInfo(url: string, connectionInfo: ConnectionInfo): void;
    @event devices(url: string, devices: DeviceInfo[]): void;
    @event peerConnected(url: string, peerId: string): void;
    @event peerDisconnected(url: string, peerId: string): void;
    @event authAnonymous(signalingUrl: string, authDomain: string): void;
    @event deviceRegistered(url: string, deviceId: string, tags?: Record<string>): void;
    @event deviceDeregistered(url: string, deviceId: string): void;
    @event tagsUpdated(url: string, deviceId: string, tags: Record<string>): void;
    @event deviceUpdated(url: string, deviceId: string, tags: Record<string>, deviceConfig?: unknown): void;
    @event pairingCode(url: string, code: string): void;
    @event pairingConnected(url: string, peerId: string): void;
    @event pairingFailed(url: string, reason: string): void;
    @event syncData(url: string, payload: unknown): void;
    @event roomUpdated(url: string, room: RoomInfo): void;
    @event roomActorJoined(url: string, roomId: string, deviceId: string): void;
    @event roomActorLeft(url: string, roomId: string, deviceId: string): void;
    @event roomMessage(url: string, roomId: string, from: string, payload: unknown): void;
}

// ── EventBus: Auth ─────────────────────────────────────────

@bus("adi.auth")
interface AuthBus {
    @event stateChanged(user: unknown): void;
    @event getToken(authDomain: string, sourceUrl?: string): void;
    @event tokenResolved(authDomain: string, token: string | null): void;
    @event sessionSave(accessToken: string, email: string, expiresAt: int32, authUrl: string): void;
}
