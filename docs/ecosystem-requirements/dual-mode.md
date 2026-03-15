# Dual Mode Output

## Why This Rule Exists

ADI runs in two contexts: interactive terminal (human user) and WebRTC/cloud (`SILK_MODE=true`, consumed by machines). All output must work in both modes. `lib-console-output` handles this automatically -- but only if you use it.

## How It Works

| Mode | Trigger | Output Format |
|------|---------|---------------|
| Text mode | Default | Colored text, Unicode icons, formatted tables |
| JSON stream mode | `SILK_MODE=true` | Structured JSON events, one per line |

All `lib-console-output` block components (`Section`, `KeyValue`, `Table`, `Columns`, `List`, `Card`) and macros (`out_info!`, `out_error!`, etc.) automatically adapt. Input components (`Select`, `Confirm`, `Input`) emit JSON prompts and accept JSON responses in silk mode.

## Rules

### 1. Use `lib-console-output` Exclusively

This is the only way to guarantee dual-mode support. Raw `println!` produces nothing in JSON stream mode.

### 2. Don't Branch on Mode Manually

```rust
// BAD
if silk_mode() {
    println!("{}", serde_json::to_string(&data)?);
} else {
    pretty_print(&data);
}

// GOOD
KeyValue::new()
    .entry("Name", &data.name)
    .entry("Status", &data.status)
    .print();
```

The components handle mode detection internally.

### 3. Progress Indicators Are Mode-Aware

`spinner()`, `progress_bar()`, `steps()`, and `MultiProgress` emit structured JSON progress events in silk mode. Don't skip progress reporting because "it's only for terminals."

### 4. Errors Must Be Structured

```rust
// BAD - invisible in JSON mode
eprintln!("failed to connect: {}", err);

// GOOD - emits structured error event
out_error!("Failed to connect: {}", err);
```

## The Test

*"Would a WebRTC client consuming JSON stream output get meaningful data from this code path?"* If you used raw print, the answer is no.
