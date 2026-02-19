# Implement Common Traits

## Why This Rule Exists

Rust's ecosystem expects types to implement standard traits. A type without `Debug` can't be printed in error messages. A type without `Clone` can't be easily duplicated. A type without `PartialEq` can't be used in assertions.

Implementing common traits makes your types work seamlessly with the standard library, testing frameworks, serialization libraries, and other code.

## Essential Traits

| Trait | When to Implement | Why |
|-------|-------------------|-----|
| `Debug` | Always for public types | Error messages, logging, debugging |
| `Clone` | If copying makes sense | Flexibility for callers |
| `PartialEq`, `Eq` | If equality comparison is meaningful | Assertions, collections |
| `Hash` | If used as map key or in sets | HashMap/HashSet support |
| `Default` | If a sensible default exists | Builder patterns, Option::unwrap_or_default |
| `Display` | For user-facing output | Distinct from Debug (developer output) |
| `From`/`Into` | For type conversions | Idiomatic conversion, ? operator |
| `Send`, `Sync` | Don't block accidentally | Async/threading compatibility |

## In Practice

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(u64);

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub timeout: Duration,
    pub retries: u32,
}
```

## Don't Derive Blindly

- `Clone` on a type holding a file handle? Probably wrong.
- `PartialEq` on a type with interior mutability? Careful with semantics.
- `Copy` should only be for small, trivial types.

## The Test

"Can this type be used in HashMap/HashSet?" If it should be, implement `Hash + Eq`.

"Can this type be printed for debugging?" If `Debug` isn't derived, add it.

"Does this type need `?` error propagation?" Implement `From` for the error type.
