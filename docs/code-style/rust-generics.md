# Use Generics to Minimize Assumptions

## Why This Rule Exists

A function taking `&Vec<i64>` only works with vectors. A function taking `impl IntoIterator<Item = i64>` works with vectors, slices, arrays, hash sets, BTreeSets, iterators, and any custom collection implementing the trait.

Specific types force callers to convert their data. Generic bounds express exactly what capabilities you need, accepting anything that provides them. More reusable code, less friction for callers.

## In Practice

```rust
// Restrictive: only accepts &Vec<i64>
fn sum(items: &Vec<i64>) -> i64 { }

// Flexible: accepts any iterable of i64
fn sum(items: impl IntoIterator<Item = i64>) -> i64 { }
```

The generic version accepts:
- `sum(vec![1, 2, 3])`
- `sum([1, 2, 3])`
- `sum(&[1, 2, 3])`
- `sum(hashset.iter().copied())`
- `sum(1..100)`

## Trade-offs

**Benefits:**
- More reusable
- Static dispatch (monomorphization) = fast
- Compile-time verification of bounds

**Costs:**
- Increased binary size (one copy per concrete type)
- Complex bounds hurt readability
- Compile times increase with heavy generics

## When to Use Trait Objects Instead

Use `dyn Trait` when:
- You need heterogeneous collections
- Binary size matters more than speed
- You want to avoid monomorphization bloat

## The Test

"Does this function need `Vec` specifically, or does it just need to iterate?" Use the minimal bound.
