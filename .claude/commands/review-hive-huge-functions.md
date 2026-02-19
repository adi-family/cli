---
allowed-tools: Bash(adi flags *), Bash(cargo:*), Read, Grep, Glob, Edit, Task
argument-hint: [file or directory paths...]
description: Review hive crate files for huge functions and split them into smaller focused helpers
---

## Huge Functions Cleanup Review (Hive)

Review the specified hive crate files for oversized functions, then flag passing files with `adi flags set hive_huge_functions`.

**Target:** `$ARGUMENTS`

If no arguments provided, find dirty (unflagged) files in hive:
```bash
adi flags status hive_huge_functions | grep "crates/hive"
```

Or list all hive Rust files:
```bash
find crates/hive -name "*.rs" -type f
```

### Thresholds

| Metric | Threshold | Severity |
|--------|-----------|----------|
| Function body lines | > 40 lines | Must split |
| Function body lines | > 25 lines | Review — split if doing multiple things |
| Parameters | > 5 | Extract into a config/options struct |
| Nesting depth | > 3 levels | Extract inner blocks into helpers |
| Cyclomatic complexity | > 10 branches | Split into smaller decision functions |

### Review Process

For each file:

1. **Read the file** completely
2. **Identify** all functions/methods exceeding the thresholds
3. **Analyze** each oversized function:

#### Single Responsibility
- Does the function do exactly one thing?
- Can you describe what it does without using "and"?
- If it has phases/stages, each phase should be its own function

#### Extraction Candidates
- Sequential blocks of logic (setup, process, cleanup)
- Conditional branches with substantial bodies (> 5 lines per arm)
- Loop bodies with complex logic
- Error handling blocks that could be helper functions
- Repeated patterns across match arms

#### Naming After Split
- Helper names should describe *what* they do, not *when* they run
- Avoid generic names like `process_inner`, `do_step2`, `handle_rest`
- Good: `validate_input`, `build_response`, `apply_transformations`

#### Rust-Specific Patterns
- Extract closures into named functions when they exceed 10 lines
- Use early returns to reduce nesting before splitting
- Consider trait method decomposition for large impl blocks
- Builder pattern for functions with many parameters
- Use `?` chains and combinators to flatten error handling

#### Hive-Specific Patterns
- Split service lifecycle methods: `prepare_service`, `start_service`, `monitor_service`
- Extract Docker command building into separate functions
- Separate config parsing from validation from application
- Split proxy middleware into request/response phases
- Extract health check logic from retry/backoff logic
- Plugin initialization should be separate from plugin execution

### Output Format

For each file, report:
- **File path**
- **Verdict**: PASS or NEEDS_WORK
- **Functions exceeding thresholds**: name, line count, specific issues

### After Review

1. **Refactor** all NEEDS_WORK functions by splitting them
2. **Verify** refactored code compiles: `cargo check -p hive-core` or `cargo check -p hive-http` or `cargo check -p hive-plugin`
3. **Verify** behavior preserved — no logic changes, only structural
4. **Flag** all passing files:
   ```bash
   adi flags set hive_huge_functions <file1> <file2> ...
   ```

### Important
- Only flag files where ALL functions are within thresholds
- Preserve existing behavior — this is purely structural refactoring
- If splitting a function requires changing the public API, skip flagging and report why
- When splitting, keep the original function as the orchestrator that calls helpers
- Helpers should be private unless there's a reason to expose them
- Skip generated files in `http/src/generated/` — do not review or flag them
