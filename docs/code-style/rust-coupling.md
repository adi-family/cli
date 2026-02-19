# Loose Coupling

## Why This Rule Exists

Tight coupling makes code rigid. When component A directly depends on the concrete implementation of component B, changes to B ripple through A. Testing A requires real instances of B. Replacing B requires rewriting A. The system becomes a monolith in disguise.

Loose coupling inverts this: A depends on an interface (trait), and B happens to implement that interface. A can be tested with mocks. B can be swapped for alternative implementations. Components evolve independently.

## In Practice

- Define behavior with traits, not concrete types
- Accept `impl Trait` or generics in function signatures
- Use dependency injection: pass collaborators via constructors, not global state
- Keep modules focused -- each crate should have a single responsibility

## Struct Decomposition Pattern

Split large structs into smaller, focused pieces. This solves borrow checker issues and creates natural seams for testing:

```rust
// Instead of one giant Config struct:
struct Config {
    connection_string: ConnectionString,
    timeout: Timeout,
    pool: PoolConfig,
}

// Each piece can be borrowed independently
// Each piece can be tested independently
// Each piece can be reused in different contexts
```

## The Test

"Can I test this component in isolation?" If no, there's too much coupling.

"If I change the implementation of a dependency, does this code need to change?" If yes, you're depending on implementation details.
