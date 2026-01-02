flowmap-api, rust, http, code-visualization, multi-language

## Overview
- HTTP API server for FlowMap code visualization
- Parses codebases into visual flow graphs and data dependency blocks
- Multi-language: TypeScript, JavaScript, Python, Java
- Standalone build (not in workspace due to tree-sitter version conflicts)

## Build
```bash
cd apps/flowmap-api
cargo build --release
```

## Run
```bash
# Default port 8092
./target/release/flowmap-api

# Custom port
PORT=8080 ./target/release/flowmap-api
```

## API Endpoints

### V1 API (FlowGraph format - edges/nodes)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/api/parse?path=/path/to/dir` | Parse a directory, returns flow summaries |
| GET | `/api/flows?path=/path/to/dir` | List flows for a parsed directory |
| GET | `/api/flows/{id}?path=/path/to/dir` | Get full flow graph by ID |
| GET | `/api/flows/{id}/issues?path=/path/to/dir` | Get issues (unhandled errors, etc.) |
| GET | `/api/source/{node_id}?path=/path/to/dir` | Get source location for a node |

### V2 API (Block format - flat library with data flow)

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v2/parse?path=/path/to/dir` | Parse directory into block library |
| POST | `/api/v2/parse/file` | Parse single file from source |
| GET | `/api/v2/blocks?path=/path/to/dir` | Get full block output |
| GET | `/api/v2/blocks/{id}?path=/path/to/dir` | Get specific block by ID |
| GET | `/api/v2/languages` | Get supported languages |

## Response Format
```json
{
  "success": true,
  "data": { ... },
  "error": null
}
```

## V2 Block Output Format
```json
{
  "library": {
    "block_1": {
      "name": "create_order",
      "type": "function",
      "uses_data": ["user_id", "items"],
      "produces_data": ["order"],
      "children": ["block_2", "block_3"]
    },
    "block_2": {
      "name": "get_user",
      "type": "call",
      "uses_data": ["user_id"],
      "produces_data": ["user"],
      "children": []
    }
  },
  "root": ["block_1"],
  "file": "example.ts",
  "language": "typescript"
}
```

## Block Types
- Entry: module, class, function, async_function, method, async_method, arrow, generator
- Control: if, else, else_if, switch, case, default, try_catch, try, catch, finally
- Loops: for, for_in, for_of, for_await, while, do_while
- Expressions: call, method_call, await_call, new, assignment, destructure
- Returns: return, throw, break, continue, yield, yield_from
- Declarations: variable, const, let, parameter, property
- Class: constructor, static_method, static_property, getter, setter
- Types: interface, type_alias, enum, enum_member (TypeScript)

## Supported Languages
- TypeScript (.ts, .tsx)
- JavaScript (.js, .jsx, .mjs, .cjs)
- Python (.py, .pyw)
- Java (.java)

## Data Flow Tracking
- `uses_data`: Variables/identifiers consumed (read) by this block
- `produces_data`: Variables/identifiers created (written) by this block
- `children`: References to child blocks in the library (hierarchical structure)

## Entry Points Detected
- HTTP handlers (Express-style: `app.get()`, `app.post()`, etc.)
- Exported functions
- Class definitions
- Decorated functions (Python decorators, TypeScript decorators)
