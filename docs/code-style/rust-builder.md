# Builder Pattern for Complex Construction

## Why This Rule Exists

Constructors with many parameters are hard to read and easy to misuse:
```rust
Server::new("0.0.0.0", 8080, true, false, Some(30), None, vec![], true)
```

What do those booleans mean? Which `None` is which? Argument order bugs compile successfully.

Builders provide named setters, optional parameters with defaults, and validation before construction:
```rust
Server::builder()
    .host("0.0.0.0")
    .port(8080)
    .tls(true)
    .timeout(Duration::from_secs(30))
    .build()?
```

## In Practice

Two patterns exist:

**Non-consuming (preferred)**: Methods take `&mut self`, return `&mut Self`. Terminal method borrows `&self`. Allows both one-liners and incremental construction.

**Consuming**: Methods take `self`, return `Self`. Required when construction transfers ownership (like spawning a process).

```rust
// Non-consuming: flexible
impl ServerBuilder {
    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }
    pub fn build(&self) -> Result<Server, Error> { }
}

// Usage: one-liner or incremental
let server = ServerBuilder::new().port(8080).build()?;

let mut builder = ServerBuilder::new();
if ssl { builder.tls(true); }
builder.build()?
```

## When to Use

- More than 3-4 constructor parameters
- Optional parameters with sensible defaults
- Configuration that may grow over time
- Validation required before construction

## The Test

"Is this constructor getting unwieldy?" If yes, introduce a builder.
