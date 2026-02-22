- **Always use `lib-console-output`** for all terminal output -- never use raw `println!`/`eprintln!`
  - Use `out_info!`, `out_success!`, `out_warn!`, `out_error!`, `out_debug!` macros for messages
  - Use `Section` for headers, `Columns`/`Table` for tabular data, `List` for bullet lists, `KeyValue` for label-value pairs
  - Use `theme::*` functions for styling (`theme::success`, `theme::error`, `theme::brand_bold`, etc.)

- **KISS**: Simple code over clever code. Code exists for humans. Don't import enterprise patterns from other languages. If you need a comment to explain what code does, simplify the code instead.

- **DRY**: Extract repeated logic, but wait for the third occurrence. Premature abstraction creates worse coupling than duplication. Use traits and generics as primary abstraction tools.

- **YAGNI**: Don't implement speculative features. Rust's traits eliminate many OO patterns (Strategy, Factory, Observer). Refactoring is cheap -- add abstraction when you need it.

- **Loose coupling**: Depend on traits, not concrete types. Accept `impl Trait` or generics. Use dependency injection. Split large structs for independent borrowing and testing.

- **Small crates**: One responsibility per crate. Core logic in libraries, thin wrappers for CLI/HTTP/plugin. Enables parallel compilation and code reuse.

- **Borrowed types**: Prefer `&str` over `&String`, `&[T]` over `&Vec<T>`. More flexible for callers, fewer indirections.

- **Newtype pattern**: Wrap primitives in single-field structs for type safety. `Miles(f64)` vs `Kilometers(f64)` catches bugs at compile time, zero runtime cost.

- **Custom types over bool**: Use enums (`Size::Small`) instead of booleans. Self-documenting, extensible, catches argument-order bugs.

- **Generics**: Accept `impl IntoIterator<Item = T>` over `&Vec<T>`. Express minimal requirements, accept maximum inputs.

- **Builder pattern**: For types with many optional parameters. Named setters, defaults, validation. Prefer non-consuming builders (`&mut self`) for flexibility.

- **Avoid Deref abuse**: `Deref` is for smart pointers, not inheritance. Use composition + explicit delegation or traits instead.

- **Avoid Clone abuse**: Don't sprinkle `.clone()` to silence borrow checker. Restructure borrows, scope them tightly, or decompose structs. Clone hides design problems.

- **Extensibility**: Use `#[non_exhaustive]` or private fields to allow adding fields/variants without breaking changes.

- **Error handling**: Specific enum variants, preserved error chains (`#[source]`), actionable context (paths, values). Document with `# Errors` section.

- **Common traits**: Always implement `Debug`. Add `Clone`, `PartialEq`, `Hash`, `Default` where meaningful. Don't block `Send`/`Sync` accidentally.

- **Documentation**: First line = summary. Add `# Examples`, `# Errors`, `# Panics`, `# Safety` sections as needed. Use `?` in examples, not `unwrap()`.

- **Module structure**: When a subdirectory contains only 2 files (`mod.rs` + one impl), flatten to sibling files: `foo/mod.rs` + `foo/bar.rs` → `foo.rs` + `foo_bar.rs`. Use `#[path = "foo_bar.rs"] mod bar;`. Subdirectories justified with 3+ files.
