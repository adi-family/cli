# Use Borrowed Types for Arguments

## Why This Rule Exists

`&str` accepts more types than `&String`. Through deref coercion, `&str` accepts `&String`, `&str`, string literals, and substrings. `&String` only accepts `&String`, requiring callers to convert.

Additionally, `&String` has two levels of indirection (pointer to String, which points to heap). `&str` has one level (pointer directly to bytes). Fewer indirections = better cache performance.

The same logic applies to `&[T]` vs `&Vec<T>` and `&T` vs `&Box<T>`.

## In Practice

```rust
// Preferred: accepts &String, &str, string literals
fn process(data: &str) { }

// Restrictive: only accepts &String
fn process(data: &String) { }
```

The first version works with:
- `process(&my_string)` 
- `process("literal")`
- `process(&my_string[start..end])`

The second version rejects literals and slices.

## When to Use Owned Types

Take ownership (`String`, `Vec<T>`) when you need to:
- Store the value in a struct
- Transfer ownership to another thread
- Modify and return the value

If you only need to read the data, borrow the most general form.

## The Rule

"Is this a read-only operation?" If yes, use borrowed types (`&str`, `&[T]`, `&Path`).

"Do I need to own this data after the function returns?" If no, use borrowed types.
