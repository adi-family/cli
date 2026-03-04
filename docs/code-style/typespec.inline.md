# TypeSpec Code Generation

Use `.tsp` files as the single source of truth for wire protocol types. The `tsp-gen` tool generates both Rust and TypeScript from the same definition.

## Crate Structure

```
crates/tsp-gen/
  core/     # Parser, AST, code generators (lib-typespec-api)
  plugin/   # ADI CLI plugin (adi tsp-gen)
```

## Protocol Generation

Protocol mode (`-s protocol`) generates discriminated union types from `@channel` interfaces.

### TypeSpec Definition

```typespec
enum AuthRequirement { required: "required", optional: "optional" }

model ConnectionInfo { manual_allowed: boolean; }

@channel("auth")
interface Auth {
    @serverPush hello(auth_kind: string, auth_requirement: AuthRequirement): void;
    @request authenticate(access_token: string): { user_id: string; };
}

@channel("device")
interface Device {
    @request register(secret: string, device_id?: string): { device_id: string; };
    @event peerConnected(peer_id: string): void;
}
```

### Decorators

| Decorator | Generates |
|-----------|-----------|
| `@request` | Request variant + response variant |
| `@event` | Single event variant |
| `@serverPush` | Server-to-client push variant |
| `@relay` | Relay variant (forwarded as-is) |
| `@scatter` | Scatter variant (broadcast) |

### Rust Output

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SignalingMessage {
    AuthHello { auth_kind: String, auth_requirement: AuthRequirement },
    AuthAuthenticate { access_token: String },
    AuthAuthenticateResponse { user_id: String },
    DeviceRegister { secret: String, device_id: Option<String> },
    DeviceRegisterResponse { device_id: String },
    DevicePeerConnected { peer_id: String },
}
```

### TypeScript Output

```typescript
export type SignalingMessage =
  // ── auth ──
  | { type: 'auth_hello'; auth_kind: string; auth_requirement: AuthRequirement }
  | { type: 'auth_authenticate'; access_token: string }
  | { type: 'auth_authenticate_response'; user_id: string }
  // ── device ──
  | { type: 'device_register'; secret: string; device_id?: string }
  | { type: 'device_register_response'; device_id: string }
  | { type: 'device_peer_connected'; peer_id: string };
```

## CLI Usage

```bash
# TypeScript protocol
adi tsp-gen generate signaling.tsp -l typescript -s protocol \
  --protocol-tag type --protocol-rename snake_case --protocol-enum-name SignalingMessage \
  -o apps/app/src/app/generated/signaling

# Rust protocol (usually via build.rs instead)
adi tsp-gen generate signaling.tsp -l rust -s protocol \
  --protocol-tag type --protocol-rename snake_case --protocol-enum-name SignalingMessage \
  -o crates/lib/lib-signaling-protocol/src/generated

# HTTP API generation
adi tsp-gen generate api.tsp -l typescript -s client -o src/generated
adi tsp-gen generate api.tsp -l rust -s server -o src/generated
adi tsp-gen generate api.tsp -l python -s both -o src/generated
adi tsp-gen generate api.tsp -l openapi -o docs/api
```

## HTTP API Generation

HTTP mode (`-s client`, `-s server`, `-s both`) generates typed clients and server traits from `@route` interfaces.

### TypeSpec Definition

```typespec
model User {
  id: uuid;
  email: email;
  name?: string;
}

model EmailLoginRequest {
  email: email;
}

model TokenResponse {
  accessToken: string;
  tokenType: "Bearer";
  expiresIn: int32;
  refreshToken?: string;
}

@route("/auth")
interface AuthService {
  @post @route("/login/email")
  loginWithEmail(@body body: EmailLoginRequest): { @statusCode statusCode: 200; @body body: TokenResponse; };

  @get @route("/me")
  getCurrentUser(): { @statusCode statusCode: 200; @body body: User; };
}
```

### HTTP Decorators

| Decorator | Purpose |
|-----------|---------|
| `@route("/path")` | URL path prefix or segment |
| `@get`, `@post`, `@put`, `@patch`, `@delete` | HTTP method |
| `@body` | Request/response body |
| `@path` | Path parameter (e.g. `@path id: string`) |
| `@query` | Query parameter |
| `@header` | Header parameter |
| `@statusCode` | Response status code |

### Rust Server Output (`-s server`)

Generates axum router + handler trait:

```rust
#[async_trait]
pub trait AuthServiceApi: Send + Sync + 'static {
    async fn login_with_email(&self, body: EmailLoginRequest) -> Result<TokenResponse, ApiError>;
    async fn get_current_user(&self) -> Result<User, ApiError>;
}

pub fn auth_service_router<S, T>(state: State<Arc<T>>) -> Router<S>
where T: AuthServiceApi { ... }
```

### TypeScript Client Output (`-s client`)

Generates fetch-based client class:

```typescript
export class AuthServiceClient extends BaseClient {
  async loginWithEmail(body: EmailLoginRequest): Promise<TokenResponse> { ... }
  async getCurrentUser(): Promise<User> { ... }
}
```

### Python Client Output (`-s client`)

```python
class AuthServiceClient(BaseClient):
    async def login_with_email(self, body: EmailLoginRequest) -> TokenResponse: ...
    async def get_current_user(self) -> User: ...
```

### OpenAPI Output (`-l openapi`)

Generates OpenAPI 3.0 spec in both JSON and YAML.

## build.rs Integration (Rust)

For Rust crates, use `build.rs` for protocol generation — it runs automatically during `cargo build`:

```rust
use typespec_api::codegen::{protocol::RustProtocolConfig, Generator, Language, Side};

fn main() {
    println!("cargo:rerun-if-changed=signaling.tsp");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let source = std::fs::read_to_string("signaling.tsp").expect("read .tsp");
    let file = typespec_api::parse(&source).expect("parse .tsp");

    Generator::new(&file, &format!("{out_dir}/protocol").as_ref(), "signaling")
        .with_rust_protocol_config(RustProtocolConfig {
            tag: "type".to_string(),
            rename: "snake_case".to_string(),
            enum_name: "SignalingMessage".to_string(),
        })
        .generate(Language::Rust, Side::Protocol)
        .expect("codegen failed");
}
```

## Hive Integration

Use pre-up hooks to regenerate TypeScript types before services start:

```yaml
signaling:
  hooks:
    pre-up:
      - type: script
        run: >-
          adi tsp-gen generate
          crates/lib/lib-signaling-protocol/signaling.tsp
          -l typescript -s protocol
          --protocol-tag type --protocol-rename snake_case --protocol-enum-name SignalingMessage
          -o apps/app/src/app/generated/signaling
```

## Protocol Options

| Flag | Default | Description |
|------|---------|-------------|
| `--protocol-tag` | `type` | Discriminant field name |
| `--protocol-rename` | `snake_case` | Wire name strategy: `snake_case`, `camelCase`, `PascalCase` |
| `--protocol-enum-name` | `SignalingMessage` | Generated enum/union type name |

## Sides

| Side | Description |
|------|-------------|
| `client` | Client SDK (fetch-based) |
| `server` | Server traits + router (axum) |
| `both` | Client + server |
| `types` | Models + enums only |
| `protocol` | Discriminated union from `@channel` interfaces |
| `adi` | AdiService implementation for WebRTC transport |
