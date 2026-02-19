# DRY (Don't Repeat Yourself)

## Why This Rule Exists

Duplicated code means duplicated bugs. When behavior needs to change, you fix it in one place or you miss one and create inconsistency. Maintenance becomes a scavenger hunt through the codebase.

But DRY is a goal, not a religion. Premature abstraction creates coupling that's worse than duplication. The wrong abstraction forces all callers through a shared interface that serves none of them well. "A little copying is better than a little dependency."

## In Practice

- Wait for the third occurrence before abstracting (Rule of Three)
- Use traits for shared behavior across types -- this is Rust's primary abstraction mechanism
- Use generics when multiple types need the same logic with different concrete types
- Extract functions when the same sequence of operations appears in multiple places
- Use macros only when traits/functions can't express the pattern (e.g., compile-time code generation)

## When Duplication Is Acceptable

- Two copies that look similar now but may evolve differently
- Test code where clarity matters more than DRY
- Configuration that happens to match today but shouldn't be coupled
- When the shared abstraction would require passing many context parameters

## The Test

Before abstracting: "Would a change to one copy necessarily require the same change to the other?" If no, keep them separate.
