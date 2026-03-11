plugin-authoring, typespec, adi-service, codegen, cocoon

## Plugin Structure

```
plugins/adi.<name>/
  api.tsp                    # Source of truth for API
  core/
    Cargo.toml
    src/
      lib.rs                 # Types, business logic, re-exports
      models/                # Domain types used by generated handler
  plugin/
    Cargo.toml
    build.rs                 # Calls PluginWebBuild (generates TS types + adi-client)
    src/lib.rs               # ADI CLI plugin entry point
  web/
    src/
      generated/             # 100% auto-generated from api.tsp (safe to delete + rebuild)
        types.ts
        enums.ts
        adi-client.ts        # Typed API client functions
        index.ts
      plugin.ts              # Hand-written: UI, lifecycle, bus events
      types.ts               # Hand-written: extends generated types (e.g. adds cocoonId)
  migrations/                # SQL migrations
```

## How It Works

Write `api.tsp` with `@channel` + `@request` decorators, implement handler trait, done.

### 1. Define API in TypeSpec (`api.tsp`)

```typespec
@channel("adi.my-plugin")
interface MyService {
  @request list(filter?: string): Item[];
  @request get(id: uuid): Item;
  @request create(name: string, data: Record<unknown>): Item;
}
```

- `@channel("adi.my-plugin")` sets the plugin_id and wire channel name
- `@request` marks methods as request/response operations
- Method names: `camelCase` in TypeSpec, `snake_case` on wire, `camelCase` in TS client

### 2. Rust Handler Generation (cocoon-core build.rs)

The handler trait + AdiService wrapper are generated to `OUT_DIR` via cocoon-core's `build.rs`:

```rust
// In cocoon-core build.rs:
let adi_config = RustAdiServiceConfig {
    types_crate: "my_plugin_core".into(),  // crate with models
    cocoon_crate: "crate".into(),          // cocoon-core itself
    service_name: "MyPlugin".into(),
    ..Default::default()
};
Generator::new(&file, &out_dir, "my_plugin")
    .with_rust_adi_config(adi_config)
    .generate(Language::Rust, Side::AdiService)
    .expect("codegen");
```

This generates:
- `MyPluginServiceHandler` trait with typed methods
- `MyPluginServiceAdi<H>` wrapper implementing `AdiService`

### 3. Implement the Handler (in cocoon-core)

```rust
// In cocoon-core service file:
include!(concat!(env!("OUT_DIR"), "/my_plugin_adi_service.rs"));

pub struct MyPluginService { /* business logic deps */ }

#[async_trait]
impl MyPluginServiceHandler for MyPluginService {
    async fn list(&self, ctx: &AdiCallerContext, filter: Option<String>)
        -> Result<Vec<Item>, AdiServiceError> {
        // business logic
    }
    // ...
}
```

Register with the router:
```rust
let svc = MyPluginServiceAdi::new(MyPluginService::new());
router.register(Arc::new(svc));
```

### 4. TypeScript Client Generation (plugin build.rs)

```rust
// In plugin/build.rs:
fn main() {
    lib_plugin_web_build::PluginWebBuild::new()
        .tsp_path("../api.tsp")
        .run();
}
```

Generates `web/src/generated/adi-client.ts` with typed functions:
```typescript
export const list = (c: Connection, params?: { filter?: string }) =>
  c.request<Item[]>(SVC, 'list', params ?? {});

export const get = (c: Connection, id: string) =>
  c.request<Item>(SVC, 'get', { id });
```

## Key Rules

- `web/src/generated/` is 100% auto-generated, never hand-edit
- Generated Rust goes to `OUT_DIR` via `include!()`, never committed
- Wire format uses `snake_case` method names everywhere
- TS function names use `camelCase` (JS convention)
- Core crate must re-export `Uuid` and `DateTime<Utc>` in `models` module for generated code
- Core crate must have `pub mod enums` re-exporting enum types for generated imports
- Handler trait lives in cocoon-core (not plugin core) due to circular dependency constraint

## Models Module Setup

The core crate `models/mod.rs` must re-export types the generated handler uses:

```rust
mod my_model;

pub use chrono::{DateTime, Utc};
pub use my_model::*;
pub use uuid::Uuid;
```

And `lib.rs` needs an `enums` module:

```rust
pub mod enums {
    pub use super::models::MyEnum;
}
```

## TypeSpec Decorator Reference

| Decorator | Purpose |
|-----------|---------|
| `@channel("adi.plugin-id")` | Sets plugin_id and wire channel |
| `@request` | Request/response method |
| `@event` | One-way event (protocol mode only) |
| `@serverPush` | Server-to-client push (protocol mode only) |

## Existing Plugin: adi.credentials

Reference implementation in `plugins/adi.credentials/`.
- `api.tsp`: 8 methods (list, get, getWithData, create, update, delete, verify, accessLogs)
- Handler: `plugins/adi.cocoon/core/src/services/credentials.rs`
- Web client: `plugins/adi.credentials/web/src/generated/adi-client.ts`
