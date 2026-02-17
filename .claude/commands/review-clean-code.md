---
allowed-tools: Bash(adi flags *), Bash(cargo:*), Read, Grep, Glob, Edit, Task
argument-hint: [file or directory paths...]
description: Review files for clean code and readability, then flag them
---

## Clean Code & Readability Review

Review the specified files for clean code quality, then flag passing files with `adi flags set clean_code`.

**Target:** `$ARGUMENTS`

If no arguments provided, find dirty (unflagged) files:
```bash
adi flags status clean_code
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

### Output Format

For each file, report:
- **File path**
- **Verdict**: PASS or NEEDS_WORK
- **Issues** (if any): concise list with line numbers

### After Review

1. **Fix** all NEEDS_WORK issues by editing the files
2. **Verify** fixes compile: `cargo check -p <relevant-package>`
3. **Flag** all passing files:
   ```bash
   adi flags set clean_code <file1> <file2> ...
   ```

### Important
- Only flag files that genuinely pass all criteria
- Fix issues in-place rather than just reporting them
- If a file has issues you cannot fix (architectural, needs user input), skip flagging it and report why
