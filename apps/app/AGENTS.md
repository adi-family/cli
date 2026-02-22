app, vite, lit, tailwind, design-system, ax-system

## Overview
- Vite + Lit 3 + Tailwind v4 application
- Uses `rolldown-vite` as Vite override for bundling
- Imports ADID design system via `@adi-family/sdk-parts-css` and `@adi-family/sdk-ui-components`
- All UI web components from `packages/ui-components/` registered via `@adi-family/sdk-ui-components`
- Theme tokens from `packages/theme/generated/adi-theme.css`

## AX System: `--t`, `--l`, `--r`

Three cascading CSS custom properties registered via `@property` with `inherits: true`.
Self-referential calc works: `--l: calc(var(--l) * 0.5)` resolves the inherited parent value before reassignment.
This means applying a single class scales an entire subtree.

### `--t` (text sizing, default `1rem`)
Controls all font sizes and icon sizes.

| Class | Effect |
|-------|--------|
| `.t-1/2` | halves text size |
| `.t-3/4` | 75% text size |
| `.t-7/8` | 87.5% text size |
| `.t-2` | doubles text size |
| `.t-3` | triples text size |

Text utility classes (Major Third 1.25 scale): `.text-xs` (0.75x), `.text-sm` (0.875x), `.text-base` (1x), `.text-lg` (1.25x), `.text-xl` (1.563x), `.text-2xl` (1.953x), `.text-3xl` (2.441x), `.text-4xl` (3.052x).

Icon sizes: `.icon-xs`, `.icon-sm`, `.icon-1`, `.icon-lg`, `.icon-xl`.

### `--l` (layout/spacing, default `1rem`)
Controls all spacing: padding, margin, gap, element sizes.

| Class | Effect |
|-------|--------|
| `.l-1/4` | 25% of parent spacing |
| `.l-1/2` | 50% of parent spacing |
| `.l-3/4` | 75% of parent spacing |
| `.l-2` | doubles parent spacing |
| `.l-3` | triples parent spacing |

Spacing utilities (multiples of `--l`):
- Padding block: `.p-v-{025,05,075,1,15,2,3}`
- Padding inline: `.p-h-{025,05,075,1,15,2,3}`
- Gap: `.g-{025,05,075,1,15,2,3}`
- Size (w+h): `.s-{05,075,1,15,2,3,4,5}`
- Height: `.h-l-{1,15,2,3,4,5}`

### `--r` (radius, default `12px`)
Controls border radii with associated inner-radius math.

Classes: `.r-sm` (6px), `.r-md` (8px), `.r-lg` (12px), `.r-xl` (16px), `.r-2xl` (24px), `.r-pill` (9999px).
- `.rounded` -- applies `border-radius: var(--r)`
- `.rounded-p-{025,05,075,1,15,175,2}` -- sets radius + padding + tracks via `--r-pad`
- `.rounded-inner` -- auto-computes `max(0, --r - --r-pad)` for children

### Shortcut Combos
| Class | `--l` | `--t` |
|-------|-------|-------|
| `.compact` | 0.75x | 0.875x |
| `.spacious` | 1.5x | 1.125x |
| `.dense` | 0.5x | 0.75x |

### Usage

```html
<!-- Scale down an entire sidebar -->
<aside class="compact">
  <nav class="p-v-05 g-025 text-sm">Tighter spacing + smaller text</nav>
</aside>

<!-- Nested scaling: each .l-1/2 halves inherited --l -->
<div class="l-2">
  <div class="p-v-1">32px padding</div>
  <div class="l-1/2">
    <div class="p-v-1">16px padding (halved)</div>
  </div>
</div>

<!-- Concentric radii -->
<div class="r-2xl rounded-p-05">
  <img class="rounded-inner" src="..." />
</div>
```

UI components (buttons, inputs, toggles, etc.) inherit `--t` and `--l` from context -- they have **no size props**. Wrap them in `.compact` or `.dense` to scale down, `.spacious` or `.l-2` / `.t-2` to scale up.

## Design System Packages
| Package | npm Name | Role |
|---------|----------|------|
| `packages/css/` | `@adi-family/sdk-parts-css` | AX system, CSS utilities, component snippets |
| `packages/theme/` | -- | Theme JSON source, generated CSS/Rust outputs (10 themes) |
| `packages/ui-components/` | `@adi-family/sdk-ui-components` | Lit 3 web components using AX system |

## Scripts
- `npm run dev` -- start dev server
- `npm run build` -- typecheck + build
- `npm run preview` -- preview production build
