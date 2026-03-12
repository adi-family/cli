# ADI Crate Structure

> Auto-generate with: `adi wf generate-agents-md`

## User-Facing Components
Components with plugin for `adi` CLI integration.

| Crate | Structure | Description |
|-------|-----------|-------------|
| `agent-loop` | core,plugin | Core library for ADI Agent Loop - autonomous LLM agent with tool use |
| `analytics` | core,plugin | Core analytics types, events, errors, and migrations for ADI platform |
| `auth` | core,http,plugin | Core library for ADI Auth - email-based passwordless authentication |
| `cocoon-spawner` | core,plugin | Core library for ADI Cocoon Spawner - Docker-based cocoon lifecycle management via signaling |
| `credentials` | core,http,plugin | Core library for ADI Credentials API - secure credentials storage |
| `flags` | core,plugin | Core library for ADI file flag tracking |
| `hive` | core,plugin | Hive core library - local service orchestration business logic |
| `indexer` | core,plugin | Core indexer library for ADI - parsing, storage, search |
| `knowledgebase` | core,plugin | Core knowledgebase library for ADI - graph DB, embedding storage, semantic search |
| `linter` | core,plugin | Core library for ADI Linter - language-agnostic linting with external/plugin/command rules |
| `llm-proxy` | core,plugin | ADI LLM Proxy - Core library for LLM API proxying with BYOK/Platform modes |
| `monaco-editor` | plugin | ADI Monaco Editor plugin - web-only code editor |
| `mux` | core,plugin | Core library for ADI Mux - HTTP request fan-out multiplexer |
| `payment` | core,plugin | Core library for ADI Payment API - checkout sessions, subscriptions, and webhook handling |
| `platform` | core,http,plugin | Core library for ADI Platform API - business logic, types, and storage |
| `registry` | core,plugin | Core library for ADI plugin registry - storage and business logic |
| `signaling` | core,plugin | Core library for signaling server — state, security, token validation, and utilities |
| `tasks` | core,plugin | Core library for ADI Tasks - task management with dependency graphs |
| `tools` | core,plugin | Core library for tool index - searchable CLI tool discovery |
| `tsp-gen` | core,plugin | TypeSpec parser and multi-language code generator in pure Rust |
| `video` | core,plugin | Core library for ADI Video - programmatic video rendering with FFmpeg |
| `workflow` | plugin | ADI Workflow plugin - run shell workflows defined in TOML files |

## Backend Services
HTTP services without CLI plugin.

| Crate | Structure | Description |
|-------|-----------|-------------|


## Libraries
Shared libraries in `crates/_lib/`.

| Library | Purpose |
|---------|---------|


## Standalone Plugins

| Plugin | Description |
|--------|-------------|
| `embed-plugin` | ADI Embed plugin providing text embedding services via fastembed/ONNX |
| `llm-extract-plugin` | Extract LLM-friendly documentation from ADI plugins |
| `llm-uzu-plugin` | ADI Uzu LLM plugin for local inference on Apple Silicon |

## Tools

| Tool | Description |
|------|-------------|
| `cocoon` | Cocoon: containerized environment with signaling server connectivity for remote command execution |

## Workflows
Available workflows in `.adi/workflows/`. Run with `adi wf <name>` or directly via `.adi/workflows/<name>.sh`.

| Workflow | Description |
|----------|-------------|
| `autodoc` | Generate API documentation for Rust crates with LLM enrichment and translations |
| `build-plugin` | Build and install plugins locally (no registry deploy) |
| `clean-install` | Reset ADI installation (remove all local data for clean reinstall) |
| `cocoon-images` | Build and release cocoon Docker image variants |
| `convert-sounds` | Convert raw audio files to web-optimized MP3 and OGG formats |
| `deploy` | Deploy services to Coolify |
| `generate-agents-md` | Generate AGENTS.md and CLAUDE.md with crate structure documentation |
| `lint-plugin` | Lint and validate a plugin before release |
| `patch` | Build and patch CLI binary or plugin locally (with macOS codesign) |
| `release` | Release CLI binary or plugin |
| `seal` | Commit and push all changes including submodules |
| `sync-theme` | Sync theme JSON to CSS + Rust outputs |

## Code Style Guidelines


- **Always use `lib-console-output`** for all terminal output -- never use raw `println!`/`eprintln!`
  - Use `out_info!`, `out_success!`, `out_warn!`, `out_error!`, `out_debug!` macros for messages
  - Use `Section` for headers, `Columns`/`Table` for tabular data, `List` for bullet lists, `KeyValue` for label-value pairs
  - Use `theme::*` functions for styling (`theme::success`, `theme::error`, `theme::brand_bold`, etc.)

- **KISS**: Simple code over clever code. Code exists for humans. Don't import enterprise patterns from other languages. If you need a comment to explain what code does, simplify the code instead.

- **DRY**: Extract repeated logic, but wait for the third occurrence. Premature abstraction creates worse coupling than duplication. Use traits and generics as primary abstraction tools.

- **YAGNI**: Don't implement speculative features. Rust's traits eliminate many OO patterns (Strategy, Factory, Observer). Refactoring is cheap -- add abstraction when you need it.

- **Loose coupling**: Depend on traits, not concrete types. Accept `impl Trait` or generics. Use dependency injection. Split large structs for independent borrowing and testing.

- **Small crates**: One responsibility per crate. Core logic in libraries, thin wrappers for CLI/HTTP/plugin. Enables parallel compilation and code reuse.

- **Borrowed types**: Prefer `&str` over `&String`, `&[T]` over `&Vec<T>`. More flexible for callers, fewer indirections.

- **Newtype pattern**: Wrap primitives in single-field structs for type safety. `Miles(f64)` vs `Kilometers(f64)` catches bugs at compile time, zero runtime cost.

- **Custom types over bool**: Use enums (`Size::Small`) instead of booleans. Self-documenting, extensible, catches argument-order bugs.

- **Generics**: Accept `impl IntoIterator<Item = T>` over `&Vec<T>`. Express minimal requirements, accept maximum inputs.

- **Builder pattern**: For types with many optional parameters. Named setters, defaults, validation. Prefer non-consuming builders (`&mut self`) for flexibility.

- **Avoid Deref abuse**: `Deref` is for smart pointers, not inheritance. Use composition + explicit delegation or traits instead.

- **Avoid Clone abuse**: Don't sprinkle `.clone()` to silence borrow checker. Restructure borrows, scope them tightly, or decompose structs. Clone hides design problems.

- **Extensibility**: Use `#[non_exhaustive]` or private fields to allow adding fields/variants without breaking changes.

- **Error handling**: Specific enum variants, preserved error chains (`#[source]`), actionable context (paths, values). Document with `# Errors` section.

- **Common traits**: Always implement `Debug`. Add `Clone`, `PartialEq`, `Hash`, `Default` where meaningful. Don't block `Send`/`Sync` accidentally.

- **Documentation**: First line = summary. Add `# Examples`, `# Errors`, `# Panics`, `# Safety` sections as needed. Use `?` in examples, not `unwrap()`.

- **Module structure**: When a subdirectory contains only 2 files (`mod.rs` + one impl), flatten to sibling files: `foo/mod.rs` + `foo/bar.rs` → `foo.rs` + `foo_bar.rs`. Use `#[path = "foo_bar.rs"] mod bar;`. Subdirectories justified with 3+ files.


- **Crate structure convention:**
  ```
  crates/<component>/
    core/     # Business logic, types, traits (lib)
    http/     # REST API server - axum (bin)
    plugin/   # adi CLI plugin (cdylib)
    cli/      # Standalone CLI (bin, optional)
    mcp/      # MCP server (bin, optional)
  ```
- **Dependencies flow:** `cli` → `core` ← `http` (both depend on core)
- **Libraries** go in `crates/_lib/lib-<name>/`
- **Standalone plugins** use `crates/<name>-plugin/` pattern
- **Tools** use `crates/tool-<name>/` pattern


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
  -o crates/signaling/protocol/src/generated

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
          crates/signaling/protocol/signaling.tsp
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



**Additional guidelines:**
- [`comments`](docs/code-style/comments.md): Comments must add value.
- [`plugin-sdk`](docs/code-style/plugin-sdk.md): Schema generation
- [`rust-avoid-clone-abuse`](docs/code-style/rust-avoid-clone-abuse.md): Restructure borrows:
- [`rust-avoid-deref-abuse`](docs/code-style/rust-avoid-deref-abuse.md): Explicit delegation:
- [`rust-borrowed-types`](docs/code-style/rust-borrowed-types.md): -
- [`rust-builder`](docs/code-style/rust-builder.md): Non-consuming (preferred)
- [`rust-common-traits`](docs/code-style/rust-common-traits.md): -
- [`rust-coupling`](docs/code-style/rust-coupling.md): -
- [`rust-custom-types`](docs/code-style/rust-custom-types.md): Self-documenting code
- [`rust-documentation`](docs/code-style/rust-documentation.md): First line
- [`rust-dry`](docs/code-style/rust-dry.md): -
- [`rust-error-handling`](docs/code-style/rust-error-handling.md): Use enums for error variants:
- [`rust-extensibility`](docs/code-style/rust-extensibility.md): Benefits:
- [`rust-generics`](docs/code-style/rust-generics.md): Benefits:
- [`rust-kiss`](docs/code-style/rust-kiss.md): -
- [`rust-newtype`](docs/code-style/rust-newtype.md): Zero-cost
- [`rust-small-crates`](docs/code-style/rust-small-crates.md): Benefits:
- [`rust-yagni`](docs/code-style/rust-yagni.md): -
- [`translations`](docs/code-style/translations.md): Use Mozilla Fluent (.ftl) for all user-facing strings.
- [`ts-testing`](docs/code-style/ts-testing.md): bun:test, co-located .test.ts files, mock/spyOn patterns.
- [`web-debug-section`](docs/code-style/web-debug-section.md): Register debug sections via AdiDebugScreenBusKey.RegisterSection.

