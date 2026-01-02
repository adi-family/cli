flowmap, code-visualization, typescript-parser, flow-graphs, nestjs

## Overview
FlowMap parses TypeScript/JavaScript codebases into visual flow graphs showing control flow, function calls, and error handling paths. Supports both Express-style and NestJS decorator-based frameworks.

## Architecture

```
lib-flowmap-core     # Core types (FlowGraph, FlowNode, FlowEdge, Pin, SymbolIndex)
       ↓
lib-flowmap-parser   # TypeScript/JavaScript parser using tree-sitter
       ↓             # - TypeScriptExtractor (Express-style)
                     # - NestJsExtractor (decorator-based)
       ↓
apps/flowmap-api     # HTTP API server (Axum)
       ↓
apps/infra-service-web/src/app/flowmap   # React UI
```

## Crates

### lib-flowmap-core
Core data structures:
- `FlowGraph` - Contains nodes and edges for a single flow
- `FlowNode` - Entry points, guards, conditions, loops, calls, returns
- `FlowEdge` - Connections (execution, data, error)
- `Pin` - Input/output connection points with types
- `FlowIndex` - Collection of flows from a parsed directory
- `FlowSummary` - Lightweight flow metadata for listing
- `SymbolIndex` - Cross-file class/method resolution for NestJS
- `ClassInfo`, `MethodInfo`, `InjectionInfo` - Symbol metadata

### lib-flowmap-parser
- `FlowParser` - Main entry point, auto-detects framework
- `ParseMode` - Auto, NestJs, or Generic
- `TypeScriptExtractor` - Tree-sitter based Express/generic parsing
- `NestJsExtractor` - Two-pass NestJS parsing:
  1. Pass 1: Build SymbolIndex (classes, methods, injections, imports)
  2. Pass 2: Build flows with cross-file resolution
- `FlowAnnotator` - LLM annotation with Claude Haiku
- Skips: node_modules, .git, dist, .next, coverage, .turbo
- Supports: .ts, .tsx, .js, .jsx

### LLM Annotation
```rust
let annotator = FlowAnnotator::new(api_key);
annotator.annotate(&mut flow).await?;
// or for all flows:
annotator.annotate_index(&mut index).await?;
```

Each node gets a `description` field with human-readable text:
- `if (user.isAdmin)` → "Check if user has admin privileges"
- `await db.users.findById(id)` → "Fetch user from database by ID"
- `return { success: true }` → "Return success response"

### flowmap-api
Standalone HTTP server (not in workspace due to tree-sitter conflicts)
- Port: 8092 (default), configurable via PORT env
- State: In-memory HashMap of parsed FlowIndex per path

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /api/parse?path=` | Parse directory, returns summaries |
| `GET /api/flows?path=` | List flows (requires prior parse) |
| `GET /api/flows/{id}?path=` | Get full flow graph |
| `GET /api/flows/{id}/issues?path=` | Get issues (unhandled errors) |
| `GET /api/source/{node_id}?path=` | Get source location + content |

## Response Format
```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

## Core Types

### FlowNode
| Field | Type | Description |
|-------|------|-------------|
| `id` | `NodeId` | Unique node identifier |
| `kind` | `NodeKind` | Node type (see below) |
| `label` | `String` | Short display label |
| `code_label` | `String` | Raw code snippet |
| `description` | `Option<String>` | LLM-generated human-readable description |
| `location` | `SourceLocation` | File path + line numbers |
| `inputs` | `Vec<Pin>` | Input connection points |
| `outputs` | `Vec<Pin>` | Output connection points |

### NodeKind (tagged enum)
- Entry: `http_handler`, `event_listener`, `function_entry`, `exported_function`
- NestJS: `guard`, `pipe`, `middleware`, `interceptor`
- Control: `condition`, `merge`, `loop`, `try_catch`, `await`
- Actions: `function_call`, `method_call`, `service_call`, `repository_call`, `database_query`
- Terminals: `return`, `throw`

### Branching (if/else)
- `Condition` node has 2 output pins: `true` (pin 2), `false` (pin 3)
- Both branches are fully extracted into the graph
- `Merge` node converges branches when both continue (non-terminal)
- Merge has 2 inputs (`branch_a`, `branch_b`) and 1 output (pin 3)
- Guard clauses (early return in one branch) skip merge creation

```
[Condition: if(x)]
    ├─[true]──> [Call: a()] ─┐
    │                        │
    └─[false]─> [Call: b()] ─┤
                             ↓
                         [Merge] ──> [Call: c()]
```

### EdgeKind
- `execution` - Control flow
- `data` - Data passing (variable flow between nodes)
- `error` - Error propagation

### Data Flow Tracking
When variables are assigned from function calls and later passed as arguments:
```typescript
const user = await this.userService.findById(id);
const dto = UserDto.fromEntity(user);
return dto;
```

The parser creates **data edges** showing variable flow:
```
[ServiceCall: userService.findById] ──data:"user"──> [MethodCall: UserDto.fromEntity]
```

**Tracked expressions:**
- Simple identifiers: `user`
- Member access: `user.id` → traces back to `user`
- Method calls: `dto.toObject()` → traces back to `dto`
- Subscripts: `arr[0]` → traces back to `arr`

Edge properties:
- `kind: "data"` - Distinguishes from execution edges
- `label: "user"` - Variable name being passed
- `from_pin: 4` - Result output pin of source node
- `to_pin: 5` - Data input pin of consuming node

### Standard Pin Layout
All call nodes (ServiceCall, MethodCall, FunctionCall) use consistent pins:
| Pin | Direction | Kind | Label |
|-----|-----------|------|-------|
| 1 | Input | exec | - |
| 5 | Input | any | args (data input) |
| 2 | Output | exec | - |
| 3 | Output | error | error |
| 4 | Output | any | result |

### Deep Expression Extraction
Nested expressions are extracted into separate nodes:
```typescript
return SpotDto.fromObject(await this.service.reqById(sess, id));
```
Becomes:
```
[ServiceCall: service.reqById] → [MethodCall: SpotDto.fromObject] → [Return]
```

Arguments containing await/calls are extracted first, then the outer call.

### PinKind
- `exec` - Execution flow
- `string`, `number`, `object`, `boolean`, `any` - Data types
- `error` - Error channel

### EntryPointKind
- `HttpHandler { method, path }` - HTTP endpoints
- `EventListener { event }` - Event handlers
- `ExportedFunction` - Named exports
- `MainFunction` - Main entry

### SymbolIndex (NestJS)
- `classes` - All classes with decorators and methods
- `class_by_name` - Quick lookup by class name
- `methods` - All methods indexed by SymbolId
- `imports` - Import resolution per file
- `http_endpoints()` - Extract all HTTP routes from controllers

### ClassKind
- `Controller` - NestJS controller (@Controller)
- `Service` - Injectable service (@Injectable)
- `Repository` - Data access layer
- `Guard`, `Pipe`, `Middleware` - NestJS middleware chain
- `Entity` - TypeORM entity (@Entity)

## NestJS Detection
Auto-detects NestJS by:
1. Checking for `@Controller`, `@Injectable`, `@Module` decorators in files
2. Checking for `@nestjs/` in package.json

## Flow Example (NestJS)
For `GET /users/me`:
```
[HTTP GET /users/me]
        ↓
[Guard: JwtAuthGuard]      ← from @UseGuards
        ↓
[ServiceCall: users.reqById]
        ↓
[Condition: if (!user)]
    ├─[true]──> [Throw: NotFoundException]
    └─[false]─> [Return: MeResponseDto.fromEntity(user)]
```

## Build & Run

```bash
# Build API (standalone, from apps/flowmap-api)
cargo build --release

# Run API
PORT=8092 ./target/release/flowmap-api

# Frontend runs via infra-service-web
cd apps/infra-service-web
npm run dev
# Visit http://localhost:3000/flowmap
```

## Issue Detection
Currently detects:
- `UnhandledError` - Error pins without connections
- (Planned) `UnreachableCode`, `InfiniteLoop`, `MissingReturn`

## Layout Algorithm
Simple top-to-bottom:
1. Find entry node (no inputs)
2. BFS to assign levels
3. Center nodes horizontally per level
4. Spacing: 250px horizontal, 120px vertical

## Drill-Down (Cross-File Linking)
Service calls now have `target_flow_id` resolved for drill-down navigation:

```
[ServiceCall: users.refreshTablesByProvider]
    └─ target_flow_id: 5  → click to expand UsersService.refreshTablesByProvider flow
```

**How it works:**
1. Parser builds flows for ALL service methods (not just HTTP handlers)
2. Injection map: controller → { property_name → service_class_name }
3. For each `this.service.method()` call, resolve service class from injections
4. Link to the target method's flow via `target_flow_id`

**UI Features:**
- **Drill-down button** - Purple "Go to service.method" button in NodeInspector
- **Breadcrumb navigation** - Shows flow history, click to jump back
- **Back button** - Returns to previous flow in drill-down stack

## UI Components

### NodeInspector
Shows selected node details:
- Label, code, type, location
- Service/method info for service_call nodes
- **Drill-down button** when `target_flow_id` is available
- **Data Inputs** - Variables flowing into this node
- **Data Outputs** - Where this node's result flows
- Pin connection status

### FlowCanvas
- **Execution edges** - Solid gray lines
- **Data edges** - Dashed blue lines with variable name labels
- **Error edges** - Red lines
- Pan/zoom with mouse drag and scroll

### Breadcrumbs
When drilling down into service methods:
```
[GET /users] > [UserService.findById] > [ValidationService.check]
              ↑ click to jump back
```

## Known Limitations
- Composed decorators (`applyDecorators(Get, ...)`) not traced
- Positions recalculated on each parse
- No TypeORM query introspection (only method names)
