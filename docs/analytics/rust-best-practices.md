# Rust Best Practices
rust-style, coding-conventions, design-patterns, clean-code

---

## KISS (Keep It Simple, Stupid)

**Rule**: Prefer simple solutions over complex ones. Code exists for humans to understand, not just computers.

**Why**: Complex code is harder to maintain, debug, and extend. Simple code reduces cognitive load, makes reviews faster, and catches bugs earlier. Rust's explicit nature already encourages this -- don't fight it with unnecessary abstractions.

**In Practice**:
- Write idiomatic Rust rather than translating patterns from other languages
- Avoid over-engineering: implement what you need now, not what you might need later (see YAGNI)
- Prefer straightforward control flow over clever one-liners
- Let the type system do the heavy lifting instead of runtime checks

**Source**: [Rust Design Patterns - Idioms](https://rust-unofficial.github.io/patterns/idioms/index.html)

---

## DRY (Don't Repeat Yourself)

**Rule**: Extract repeated logic into functions, traits, or modules. But don't over-abstract prematurely.

**Why**: Duplicated code means duplicated bugs and duplicated maintenance. When behavior changes, you update one place instead of hunting through the codebase. However, premature DRY can create coupling worse than duplication.

**In Practice**:
- Use traits for shared behavior across types
- Use generics to avoid duplicating logic for different types
- Extract common patterns into helper functions
- Use macros sparingly -- only when functions/traits can't solve the problem
- Wait for the third occurrence before abstracting (Rule of Three)

**Balance**: Some duplication is acceptable if the abstractions would be worse. "A little copying is better than a little dependency."

---

## YAGNI (You Aren't Going to Need It)

**Rule**: Don't implement features or abstractions until you actually need them.

**Why**: Premature abstraction adds complexity without benefit. Rust's unique features often eliminate traditional patterns entirely. For example, the Strategy pattern is unnecessary when traits solve the problem directly.

**In Practice**:
- Implement the simplest solution that works
- Add abstractions when you have concrete use cases, not hypothetical ones
- Rust's refactoring tools make it easy to generalize later
- Many OO patterns (Factory, Strategy, Observer) are built into the language via traits

**Source**: [Rust Design Patterns - Design Patterns](https://rust-unofficial.github.io/patterns/patterns/index.html)

---

## Loose Coupling

**Rule**: Components should depend on abstractions (traits), not concrete implementations.

**Why**: Tight coupling makes code rigid, hard to test, and difficult to change. Loose coupling enables independent evolution, easier testing with mocks, and better code reuse.

**In Practice**:
- Define behavior with traits, accept trait objects or generics
- Use dependency injection via constructor parameters
- Prefer composition over inheritance (Rust doesn't have inheritance anyway)
- Keep modules focused -- small crates that do one thing well
- Use `impl Trait` in function signatures for flexibility

**Pattern**: Struct decomposition -- split large structs into smaller, independent pieces that can be borrowed and tested separately.

**Source**: [Rust Design Patterns - Compose Structs](https://rust-unofficial.github.io/patterns/patterns/structural/compose-structs.html)

---

## Prefer Small Crates

**Rule**: Create focused crates that do one thing well.

**Why**: Small crates are easier to understand, encourage modular design, enable code reuse across projects, and allow parallel compilation. The Rust ecosystem thrives on composable, single-purpose libraries.

**Trade-offs**:
- Risk of "dependency hell" with version conflicts
- No automatic LTO across crates
- Need to vet third-party code quality

**In Practice**:
- Split large projects into workspace crates
- Keep core logic in libraries, thin CLI/HTTP wrappers on top
- Follow the ADI pattern: `core/` + `http/` + `plugin/` structure

**Source**: [Rust Design Patterns - Small Crates](https://rust-unofficial.github.io/patterns/patterns/structural/small-crates.html)

---

## Use Borrowed Types for Arguments

**Rule**: Accept `&str` instead of `&String`, `&[T]` instead of `&Vec<T>`, `&T` instead of `&Box<T>`.

**Why**: Borrowed types accept more input types through deref coercion, making APIs more flexible. They also avoid unnecessary indirection -- `&String` has two layers of indirection while `&str` has one.

**In Practice**:
```rust
// Prefer this:
fn process(data: &str) { }

// Over this:
fn process(data: &String) { }
```

The first version accepts `&String`, `&str`, string literals, and `String` slices. The second only accepts `&String`.

**Source**: [Rust Design Patterns - Coercion Arguments](https://rust-unofficial.github.io/patterns/idioms/coercion-arguments.html)

---

## Newtype Pattern for Type Safety

**Rule**: Wrap primitive types in single-field tuple structs to create distinct types.

**Why**: The compiler catches logical errors at compile time. A `Miles(f64)` cannot be accidentally passed where `Kilometers(f64)` is expected. This prevents category errors like the Mars Climate Orbiter crash.

**In Practice**:
```rust
struct Miles(f64);
struct Kilometers(f64);

// Compiler error if you mix them up!
fn trip_duration(distance: Miles) -> Duration { }
```

**Benefits**:
- Zero runtime cost (same memory layout)
- Compile-time type checking
- Clean abstraction for implementation changes
- Can implement different traits for each newtype

**Source**: [Rust API Guidelines - C-NEWTYPE](https://rust-lang.github.io/api-guidelines/type-safety.html#c-newtype)

---

## Use Custom Types Over bool/Option

**Rule**: Use meaningful enum variants instead of `bool` or `Option` to convey intent.

**Why**: `Widget::new(true, false)` is unclear. `Widget::new(Small, Round)` is self-documenting. Custom types make code readable without checking function signatures.

**In Practice**:
```rust
// Clear:
let widget = Widget::new(Size::Small, Shape::Round);

// Unclear:
let widget = Widget::new(true, false);
```

**Benefits**:
- Self-documenting code
- Easier to extend (add `ExtraLarge` variant later)
- Compile-time checking prevents argument order mistakes

**Source**: [Rust API Guidelines - C-CUSTOM-TYPE](https://rust-lang.github.io/api-guidelines/type-safety.html#c-custom-type)

---

## Comments: Value Over Verbosity

**Rule**: Comments should explain *why*, not *what*. Code should be self-documenting for the *what*.

**Why**: Redundant comments become lies when code changes. Good variable names and small functions eliminate the need for most comments. Comments that explain non-obvious decisions or constraints remain valuable forever.

**In Practice**:
```rust
// Bad: restates the code
// Increment counter by one
counter += 1;

// Good: explains business logic
// Rate limit: max 100 requests per minute per client
if request_count > 100 { return Err(RateLimited); }
```

**Document**:
- Public API (always)
- Non-obvious invariants
- Performance-critical decisions
- Workarounds for external bugs

**Source**: [docs/code-style/comments.md](../code-style/comments.md)

---

## Documentation Best Practices

**Rule**: Every public item should have documentation. Use rustdoc conventions.

**Why**: Documentation is part of the API. Good docs reduce support burden, speed up onboarding, and force you to think about usability. Rust's tooling makes it easy -- `cargo doc` generates beautiful sites automatically.

**Structure**:
- **First line**: Brief description (shown in summaries)
- **# Examples**: Demonstrate common usage
- **# Errors**: When can this return `Err`?
- **# Panics**: What conditions cause panic?
- **# Safety**: For `unsafe` functions, what must the caller guarantee?

**In Practice**:
```rust
/// Parses a configuration file at the given path.
///
/// # Errors
/// Returns `ConfigError::NotFound` if the file doesn't exist.
/// Returns `ConfigError::Invalid` if the TOML is malformed.
///
/// # Examples
/// ```
/// let config = Config::from_file("config.toml")?;
/// ```
pub fn from_file(path: &str) -> Result<Config, ConfigError> { }
```

**Source**: [Rust API Guidelines - Documentation](https://rust-lang.github.io/api-guidelines/documentation.html)

---

## Use Generics to Minimize Assumptions

**Rule**: Accept the most general type that provides required functionality.

**Why**: Generic code is more reusable. A function taking `impl IntoIterator<Item = T>` works with vectors, slices, hash sets, and custom iterators. Specific types limit callers unnecessarily.

**In Practice**:
```rust
// Flexible: works with any iterable
fn sum_all<I: IntoIterator<Item = i64>>(items: I) -> i64 { }

// Restrictive: only works with Vec
fn sum_all(items: &Vec<i64>) -> i64 { }
```

**Trade-offs**:
- Generics increase code size (monomorphization)
- Complex bounds can hurt readability
- Use trait objects (`dyn Trait`) when heterogeneous collections are needed

**Source**: [Rust API Guidelines - C-GENERIC](https://rust-lang.github.io/api-guidelines/flexibility.html#c-generic)

---

## Builder Pattern for Complex Construction

**Rule**: Use builders for types that have many optional parameters or complex construction.

**Why**: Avoids constructors with many parameters, provides named setters for clarity, enables validation before construction, and supports incremental configuration.

**In Practice**:
```rust
// Clear, self-documenting construction
Command::new("git")
    .arg("commit")
    .arg("-m")
    .arg("message")
    .current_dir("/repo")
    .spawn()?;
```

**Prefer non-consuming builders** (methods take `&mut self` and return `&mut Self`) when the terminal method doesn't need ownership. This allows both one-liners and incremental configuration.

**Source**: [Rust API Guidelines - C-BUILDER](https://rust-lang.github.io/api-guidelines/type-safety.html#c-builder)

---

## Avoid Deref Polymorphism

**Rule**: Don't use `Deref` to emulate inheritance or polymorphism.

**Why**: `Deref` is for smart pointers, not type relationships. Misusing it creates surprising behavior -- methods appear from nowhere, traits don't propagate, and `self` semantics break. Future readers won't expect it.

**Instead**:
- Use composition with explicit delegation
- Implement traits on wrapper types
- Use derive macros like `delegate` or `ambassador` to reduce boilerplate

**Source**: [Rust Design Patterns - Deref Polymorphism Anti-pattern](https://rust-unofficial.github.io/patterns/anti_patterns/deref.html)

---

## Don't Clone to Satisfy the Borrow Checker

**Rule**: Understand ownership instead of throwing `.clone()` at compiler errors.

**Why**: Cloning creates independent copies. Changes aren't synchronized. This hides bugs and hurts performance. The borrow checker error is often pointing to a real design issue.

**Acceptable uses of clone**:
- Prototype/hackathon code
- Learning ownership
- Genuinely need independent copies
- `Rc`/`Arc` (designed for shared ownership)

**Better solutions**:
- Restructure code to avoid simultaneous borrows
- Use interior mutability (`RefCell`, `Mutex`) when appropriate
- Split structs for independent borrowing
- Scope borrows more tightly

**Source**: [Rust Design Patterns - Clone Anti-pattern](https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html)

---

## Extensibility via #[non_exhaustive] and Private Fields

**Rule**: Use `#[non_exhaustive]` or private fields to allow adding fields/variants without breaking changes.

**Why**: Adding a public field to a struct or variant to an enum is normally a breaking change. These mechanisms force callers to handle unknown cases, enabling forward compatibility.

**In Practice**:
```rust
#[non_exhaustive]
pub struct Config {
    pub timeout: Duration,
    pub retries: u32,
    // Can add fields later without breaking downstream
}

#[non_exhaustive]
pub enum Status {
    Active,
    Inactive,
    // Can add variants later
}
```

**Trade-offs**:
- Forces callers to use `..` in patterns
- Forces match arms with `_` wildcard
- Consider if major version bump is better

**Source**: [Rust Design Patterns - Privacy for Extensibility](https://rust-unofficial.github.io/patterns/idioms/priv-extend.html)

---

## Expose Intermediate Results

**Rule**: Return useful intermediate data from functions, not just final results.

**Why**: Callers often need related information. Returning it avoids duplicate computation.

**Examples from std**:
- `Vec::binary_search` returns the insertion index on failure, not just "not found"
- `String::from_utf8` returns the byte offset of invalid UTF-8 on error
- `HashMap::insert` returns the previous value if any

**In Practice**: When a function computes something interesting as a side effect, consider exposing it in the return type.

**Source**: [Rust API Guidelines - C-INTERMEDIATE](https://rust-lang.github.io/api-guidelines/flexibility.html#c-intermediate)

---

## Implement Common Traits

**Rule**: Types should eagerly implement standard traits where appropriate.

**Why**: Interoperability. Types that implement `Debug`, `Clone`, `PartialEq`, `Hash`, etc. integrate naturally with the ecosystem -- they work in collections, can be compared, and can be debugged.

**Core traits to consider**:
- `Debug` (always for public types)
- `Clone`, `Copy` (if semantically appropriate)
- `PartialEq`, `Eq`, `Hash` (for use in collections)
- `Default` (for builder patterns, `Option::unwrap_or_default`)
- `Display` (for user-facing output)
- `From`/`Into` (for conversions)
- `Send`, `Sync` (don't block these accidentally)

**Source**: [Rust API Guidelines - C-COMMON-TRAITS](https://rust-lang.github.io/api-guidelines/interoperability.html#c-common-traits)

---

## Error Handling: Be Meaningful

**Rule**: Error types should be specific, actionable, and preserve context.

**Why**: Good errors tell the caller what went wrong, why, and what they can do about it. `Error: something went wrong` is useless. `Error: config file not found at /path/to/config.toml` is actionable.

**In Practice**:
- Use enums for error variants, not strings
- Preserve the causal chain (use `#[source]` or `thiserror`)
- Include relevant context (file paths, parameter values)
- Implement `std::error::Error` for interoperability

**Source**: [Rust API Guidelines - C-GOOD-ERR](https://rust-lang.github.io/api-guidelines/interoperability.html#c-good-err)

---

## References

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)
- [The Rust Programming Language Book](https://doc.rust-lang.org/book/)
