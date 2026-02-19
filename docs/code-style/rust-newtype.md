# Newtype Pattern for Type Safety

## Why This Rule Exists

Primitive types carry no semantic meaning. A `f64` could be meters, miles, seconds, or dollars. Passing meters where miles is expected compiles successfully and produces wrong results silently. The Mars Climate Orbiter crashed because of exactly this bug.

Newtypes create distinct types from primitives at zero runtime cost. `Miles(f64)` and `Kilometers(f64)` are incompatible types. The compiler catches misuse at compile time, not production.

## In Practice

```rust
struct Miles(f64);
struct Kilometers(f64);
struct UserId(u64);
struct PostId(u64);

// Compiler error: expected Miles, found Kilometers
fn distance_to_destination(d: Miles) -> Duration { }

// Compiler error: expected UserId, found PostId
fn get_user(id: UserId) -> User { }
```

## Benefits

- **Zero-cost**: Same memory layout as the inner type
- **Compile-time checking**: Misuse is a type error
- **Encapsulation**: Inner representation can change without breaking API
- **Custom trait impls**: Different `Display`, `Debug`, validation per type

## When to Use

- IDs from different domains (UserId, PostId, SessionId)
- Units of measurement (Distance, Duration, Currency)
- Validated strings (Email, Url, NonEmptyString)
- Secrets that shouldn't be logged (Password, ApiKey)

## The Test

"Could mixing up values of this type with another cause bugs?" If yes, newtype it.
