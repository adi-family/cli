# Don't Clone to Satisfy the Borrow Checker

## Why This Rule Exists

When the borrow checker complains, `.clone()` makes the error disappear. But it often hides a design problem. Cloning creates independent copies -- changes to one don't affect the other. If that's not what you want, you've introduced a bug.

Excessive cloning also hurts performance. Heap allocations, memory copies, and cache pressure add up. The borrow checker error is often pointing you toward a better design.

## The Anti-pattern

```rust
// DON'T: cloning to silence the borrow checker
let data = expensive_data.clone();
process(&expensive_data);
use_data(data); // data is now stale if process() mutated expensive_data
```

## Instead Do

**Restructure borrows:**
```rust
// Scope the borrow tightly
{
    let borrowed = &expensive_data;
    process(borrowed);
}
use_data(&expensive_data); // borrow ended, can borrow again
```

**Decompose structs:** If you're borrowing parts of a struct, split it so parts can be borrowed independently.

**Interior mutability:** When shared mutable state is genuinely needed, use `RefCell`, `Mutex`, or `RwLock` explicitly.

## When Cloning Is Fine

- Small, cheap-to-copy data (`Copy` types)
- `Rc`/`Arc` (clone increments reference count, doesn't copy data)
- Prototyping code where correctness > performance
- When you genuinely need independent copies

## The Test

"Am I cloning because I need a separate copy, or because the borrow checker complained?" If the latter, find the real solution.

Run `cargo clippy` -- it catches unnecessary clones.
