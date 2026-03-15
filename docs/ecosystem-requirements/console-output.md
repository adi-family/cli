# Console Output

## Why This Rule Exists

All CLI output goes through `lib-console-output` to guarantee theme consistency, structured JSON support for WebRTC/cloud, and a unified look across every command and plugin.

## Rules

### 1. Never Use Raw Print

```rust
// BAD
println!("Done!");
eprintln!("Error: {}", e);

// GOOD
out_success!("Done!");
out_error!("Error: {}", e);

// For plain themed text
fg_println!("Some output");
```

### 2. Use the Right Macro for the Right Level

| Macro | Purpose | Icon |
|-------|---------|------|
| `out_success!` | Operation succeeded | `✓` |
| `out_error!` | Operation failed | `✕` |
| `out_warn!` | Non-fatal issue | `⚠` |
| `out_info!` | Informational | `ℹ` |
| `out_debug!` | Debug detail | `›` |
| `out_trace!` | Trace-level detail | `·` |
| `fg_println!` | General output (themed foreground) | none |

### 3. Use Block Components for Structured Data

Don't hand-format tables or lists. Use the provided components:

```rust
// Section header
Section::new("Build Results").width(50).print();

// Key-value pairs
KeyValue::new()
    .entry("Version", &version)
    .entry("Status", &theme::success("running"))
    .print();

// Tabular data with borders
Table::new()
    .header(["Name", "Status", "Port"])
    .row(["api", &theme::success("up"), "8080"])
    .print();

// Tabular data without borders
Columns::new()
    .header(["Plugin", "Version"])
    .row(["indexer", "1.2.0"])
    .print();

// Lists
List::new()
    .item("First item")
    .item("Second item")
    .numbered(false)
    .print();

// Bordered panels
Card::new()
    .title("Summary")
    .line("All checks passed")
    .print();
```

### 4. Combine Theme Functions with Blocks

Style values before passing to block components:

```rust
KeyValue::new()
    .entry("Name", &theme::brand_bold(&name))
    .entry("Status", &theme::success("active"))
    .entry("Path", &theme::muted(&path))
    .print();
```

### 5. Use Input Components for Prompts

```rust
let choice = Select::new("Pick a theme", &themes).run()?;
let confirmed = Confirm::new("Deploy now?").run()?;
let name = Input::new("Project name").run()?;
let token = Password::new("API key").run()?;
```

### 6. Progress Indicators

```rust
let sp = spinner("Building...");
// ... work ...
sp.finish("Build complete");

let bar = progress_bar(total);
bar.inc(1);
bar.finish("Done");
```

## The Test

*"Does this output look correct in both text mode and JSON stream mode?"* If you used raw print, JSON consumers get nothing.
