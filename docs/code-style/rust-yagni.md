# YAGNI (You Aren't Going to Need It)

## Why This Rule Exists

Every line of code is a liability. Speculative features add complexity, require maintenance, create potential bugs, and often end up unused or wrong when requirements actually emerge. You're paying the cost now for imaginary future value.

Rust's strong refactoring support (compiler catches breakages, cargo clippy guides improvements) means it's cheap to add abstraction later when you actually need it. Many traditional OO patterns are unnecessary because Rust's type system and traits already provide the flexibility.

## In Practice

- Implement the simplest solution that solves today's problem
- Don't add generic parameters "in case someone needs them"
- Don't create trait hierarchies for hypothetical future types
- Don't add configuration options for things only one use case needs

## Rust-Specific Wins

Rust eliminates entire pattern categories:

| Traditional Pattern | Rust Equivalent |
|---------------------|-----------------|
| Strategy Pattern | Closures or trait objects |
| Factory Pattern | Associated functions, `Default` trait |
| Observer Pattern | Channels, callbacks |
| Singleton | `once_cell::sync::Lazy`, module-level state |

Don't recreate Java/C++ patterns. Use Rust's native constructs.

## The Test

"Is there a concrete use case requiring this right now?" If no, delete it.
