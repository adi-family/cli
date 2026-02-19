# Extensibility via #[non_exhaustive]

## Why This Rule Exists

Adding a field to a public struct breaks code that constructs it directly. Adding a variant to an enum breaks exhaustive matches. These are semver-breaking changes requiring major version bumps.

`#[non_exhaustive]` prevents direct construction and exhaustive matching, reserving the right to extend later. Callers must use `..` in patterns and constructors, future-proofing their code.

## In Practice

```rust
#[non_exhaustive]
pub struct Config {
    pub timeout: Duration,
    pub retries: u32,
    // Can add fields in minor versions
}

#[non_exhaustive]
pub enum Status {
    Active,
    Inactive,
    // Can add variants in minor versions
}
```

Callers must handle unknowns:
```rust
let Config { timeout, .. } = config; // .. required
match status {
    Status::Active => {},
    Status::Inactive => {},
    _ => {}, // wildcard required
}
```

## Trade-offs

**Benefits:**
- Forward-compatible API evolution
- Minor version changes can add fields/variants

**Costs:**
- Less ergonomic for callers (forced wildcards)
- Errors for new cases appear at runtime, not compile time
- Not suitable for all enums (sometimes exhaustive is correct)

## Alternative: Private Fields

For crate-internal extensibility, a private field forces `..` patterns:
```rust
pub struct Config {
    pub timeout: Duration,
    _private: (), // prevents direct construction
}
```

## The Test

"Will this type need to grow without breaking downstream?" If yes, use `#[non_exhaustive]`.

"Is exhaustive matching a feature, not a bug?" If yes, don't use it.
