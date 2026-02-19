# Avoid Deref Polymorphism

## Why This Rule Exists

`Deref` is designed for smart pointers -- types that "contain" another type and should transparently expose it (like `Box<T>` exposing `T`). Misusing `Deref` to emulate inheritance creates surprising, hard-to-understand code.

When you implement `Deref<Target = Foo>` on `Bar`, methods from `Foo` appear on `Bar` invisibly. Future readers won't expect this. Traits on `Foo` don't transfer to `Bar`. The `self` type in methods becomes confusing.

## The Anti-pattern

```rust
// DON'T: Using Deref to inherit methods
impl Deref for Bar {
    type Target = Foo;
    fn deref(&self) -> &Foo { &self.foo }
}

// Now Bar magically has Foo's methods
bar.foo_method(); // Surprising!
```

## Instead Do

**Explicit delegation:**
```rust
impl Bar {
    fn foo_method(&self) {
        self.foo.foo_method()
    }
}
```

**Delegation macros:** Use crates like `delegate` or `ambassador` to reduce boilerplate.

**Traits:** If shared behavior is needed, define a trait and implement it for both types.

## When Deref Is Appropriate

- Smart pointers (`Box`, `Rc`, `Arc`, custom pointer types)
- Wrapper types that should expose inner type's methods (`MutexGuard`)
- Types representing "a pointer to T" semantically

## The Test

"Am I using Deref because Bar *is* a kind of pointer to Foo?" If no, don't use Deref.

"Am I trying to get inheritance?" If yes, use composition + traits instead.
