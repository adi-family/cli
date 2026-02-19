# KISS (Keep It Simple, Stupid)

## Why This Rule Exists

Complex code has compounding costs: harder to understand, harder to debug, harder to modify, harder to review. Every layer of abstraction is a layer someone must mentally unwrap. Simple code lets reviewers focus on logic, not mechanics.

Rust already enforces explicitness -- ownership, lifetimes, error handling. Fighting this with clever workarounds or over-abstraction creates friction. Embrace Rust's idioms rather than importing patterns from other languages that don't map cleanly.

The best code reads like prose. If you need a comment explaining what complex code does, that's a signal to simplify the code instead.

## In Practice

- Prefer explicit control flow (`if`/`match`) over chains of combinators when it's clearer
- Don't create abstractions until you have at least three use cases
- Split complex expressions into named intermediate variables
- If a function requires multiple paragraphs to explain, it's doing too much
- Refactor towards smaller functions, not larger ones

## Anti-patterns

- Importing "enterprise" patterns (AbstractFactoryFactory) that don't fit Rust
- Deeply nested callbacks or chains that could be flat sequential code
- Macro-heavy code when functions or traits would suffice
- Clever lifetime gymnastics when restructuring ownership would be simpler
