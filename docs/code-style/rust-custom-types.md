# Use Custom Types Over bool/Option

## Why This Rule Exists

`Widget::new(true, false)` is gibberish without checking the function signature. What does the first `true` mean? What does `false` control? Boolean parameters hide intent and invite argument-order bugs.

`Widget::new(Size::Small, Shape::Round)` is self-documenting. You can read and understand the code without context. The compiler catches if you swap the arguments.

## In Practice

```rust
// Unclear: what do these booleans mean?
fn connect(host: &str, ssl: bool, verify: bool) { }
connect("api.example.com", true, false); // ?

// Clear: intent is explicit
enum Security { Plaintext, Tls }
enum CertVerification { Verify, Skip }
fn connect(host: &str, security: Security, cert: CertVerification) { }
connect("api.example.com", Security::Tls, CertVerification::Skip);
```

## Benefits

- **Self-documenting code**: Intent is clear at call sites
- **Compile-time checking**: Can't swap arguments of different enum types
- **Extensible**: Adding `ExtraLarge` variant is easy; what's `extra_true`?
- **Refactoring safe**: Renaming variants updates all uses

## When bool Is Acceptable

- Toggle that's genuinely binary with obvious meaning: `enabled: bool`
- Builder method that sets a flag: `.compressed(true)`
- When the parameter name at call site is always visible (named arguments)

## The Test

"Reading this function call, do I know what each argument means without looking at the signature?" If no, use enums.
