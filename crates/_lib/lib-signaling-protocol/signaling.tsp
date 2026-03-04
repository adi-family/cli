// ADI Signaling Protocol — WebSocket message definitions
// Source of truth for code generation via lib-typespec-api protocol codegen.
//
// Regenerate: build.rs runs automatically on `cargo build`

// ── Supporting Types ────────────────────────────────────────

enum AuthRequirement {
    required: "required",
    optional: "optional",
}

enum AuthOption {
    verified: "verified",
    anonymous: "anonymous",
}

model ConnectionInfo {
    manual_allowed: boolean;
}

model DeviceInfo {
    device_id: string;
    tags: Record<string>;
    online: boolean;
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
    queryDevices(tag_filter: Record<string>): {
        devices: DeviceInfo[];
    };
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

// ── System Channel ──────────────────────────────────────────

@channel("system")
interface System {
    @event
    error(message: string): void;
}
