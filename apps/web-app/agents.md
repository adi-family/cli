web-app, lit, styling, tailwind, guidelines

## Styling Guidelines
- Prefer inline Tailwind classes over BEM naming conventions
- Use BEM only when truly necessary (complex component state management, third-party library integration)
- Keep styles co-located with components using Tailwind utility classes
- When writing CSS, use `@apply` with Tailwind utilities instead of raw CSS properties (e.g., `@apply w-full` not `width: 100%`)

## Icons
- Use Lucide icons for all iconography
