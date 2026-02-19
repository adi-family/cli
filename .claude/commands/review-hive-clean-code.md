---
allowed-tools: Bash(adi flags *), Bash(cargo:*), Read, Grep, Glob, Edit, Task
argument-hint: [file or directory paths...]
description: Review hive crate files for clean code and readability, then flag them
---

## Clean Code & Readability Review (Hive)

Review the specified hive crate files for clean code quality, then flag passing files with `adi flags set hive_clean_code`.

**Target:** `$ARGUMENTS`

If no arguments provided, find dirty (unflagged) files in hive:
```bash
adi flags status hive_clean_code | grep "crates/hive"
```

Or list all hive Rust files:
```bash
find crates/hive -name "*.rs" -type f
```

### Review Process

For each file:

1. **Read the file** completely
2. **Analyze** against these criteria:

#### Readability
- Clear, descriptive naming (variables, functions, types, modules)
- Functions are short and do one thing
- Code reads top-down without jumping around
- No deeply nested logic (max 3 levels)

#### Clean Code (KISS)
- No unnecessary complexity or over-abstraction
- No premature generalization
- Simple, direct solutions preferred
- No dead code or commented-out code

#### DRY
- No copy-pasted logic (3+ similar lines = extract)
- Shared behavior properly abstracted
- Constants instead of magic numbers/strings

#### Functional Style
- Prefer immutable data and pure functions
- Minimize side effects
- Use iterators/combinators over manual loops where idiomatic
- Avoid mutable state when possible

#### Rust-Specific
- Proper error handling (no unwrap in non-test code, meaningful error types)
- Idiomatic Rust patterns (Option/Result combinators, pattern matching)
- Correct use of ownership/borrowing (no unnecessary clones)
- Appropriate use of traits and generics

#### Hive-Specific
- Plugin ABI v3 compliance (use `lib-plugin-abi-v3` traits correctly)
- Proper async patterns for service management
- Docker/container lifecycle handling follows best practices
- HTTP proxy middleware chains are composable and testable
- Health check implementations are non-blocking
- Configuration parsing is robust with clear error messages

### Output Format

For each file, report:
- **File path**
- **Verdict**: PASS or NEEDS_WORK
- **Issues** (if any): concise list with line numbers

### After Review

1. **Fix** all NEEDS_WORK issues by editing the files
2. **Verify** fixes compile: `cargo check -p hive-core` or `cargo check -p hive-http` or `cargo check -p hive-plugin`
3. **Flag** all passing files:
   ```bash
   adi flags set hive_clean_code <file1> <file2> ...
   ```

### Important
- Only flag files that genuinely pass all criteria
- Fix issues in-place rather than just reporting them
- If a file has issues you cannot fix (architectural, needs user input), skip flagging it and report why
- Skip generated files in `http/src/generated/` — do not review or flag them
