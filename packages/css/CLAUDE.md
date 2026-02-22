css, design-system, adid, ax-system, components, snippets, reusable

## Overview
- Reusable CSS component snippets for all ADI websites
- Implements the ADID design system and AX cascading variable methodology
- Pure CSS classes using `--adi-*` design tokens from `packages/theme`
- No framework dependency -- works with any stack that imports the theme CSS
- Each file is a standalone snippet; import individually or via `index.css` barrel

## AX System (`ax.css`)

Three cascading CSS custom properties: `--l` (layout/spacing), `--t` (text sizing), `--r` (radius).
Registered via `@property` so `--l: calc(var(--l) * 0.5)` resolves inherited value before reassignment.

### --l context classes (spacing)
- `.l-1/4`, `.l-1/2`, `.l-3/4` -- reduce spacing in a subtree
- `.l-2`, `.l-3` -- increase spacing in a subtree

### --t context classes (text)
- `.t-1/2`, `.t-3/4`, `.t-7/8` -- reduce text in a subtree
- `.t-2`, `.t-3` -- increase text in a subtree

### Shortcut combos
- `.compact` -- `--l * 0.75, --t * 0.875`
- `.spacious` -- `--l * 1.5, --t * 1.125`
- `.dense` -- `--l * 0.5, --t * 0.75`

### Spacing utilities (--l multiples)
- `.p-v-{025,05,075,1,15,2,3}` -- padding-block
- `.p-h-{025,05,075,1,15,2,3}` -- padding-inline
- `.g-{025,05,075,1,15,2,3}` -- gap

### Text utilities (--t multiples, Major Third 1.25 scale)
- `.text-xs` (0.75x), `.text-sm` (0.875x), `.text-base` (1x)
- `.text-lg` (1.25x), `.text-xl` (1.563x), `.text-2xl` (1.953x)
- `.text-3xl` (2.441x), `.text-4xl` (3.052x)

### --r radius system (associated/concentric radii)
Inner radius = `max(0, outer_radius - padding)` for optically consistent nested curves.

- `.r-sm` (6px), `.r-md` (8px), `.r-lg` (12px), `.r-xl` (16px), `.r-2xl` (24px), `.r-pill` (9999px)
- `.rounded` -- applies `border-radius: var(--r)`
- `.rounded-p-{025,05,075,1,15,175,2}` -- sets radius + padding + tracks padding via `--r-pad`
- `.rounded-inner` -- auto-computes `max(0, --r - --r-pad)` for children

```html
<!-- Outer: 12px radius, 16px padding. Inner: max(0, 12-16) = 0px -->
<div class="r-lg rounded-p-1">
  <div class="rounded-inner">Flush inner corners</div>
</div>

<!-- Outer: 24px radius, 8px padding. Inner: max(0, 24-8) = 16px -->
<div class="r-2xl rounded-p-05">
  <img class="rounded-inner" src="..." />
</div>

<!-- Nested: radius cascades, each level subtracts its padding -->
<div class="r-2xl rounded-p-1">
  <div class="rounded-inner rounded-p-05">
    <div class="rounded-inner">Deeply nested, still smooth</div>
  </div>
</div>
```

## Files

| File | Classes | Purpose |
|------|---------|---------|
| `ax.css` | `.l-*`, `.t-*`, `.r-*`, `.rounded*`, `.compact`, `.dense`, `.spacious`, `.p-*`, `.g-*`, `.text-*` | AX three-axis cascading variable system (--l spacing, --t text, --r radius with associated inner radii) |
| `tokens.css` | (custom properties) | Standalone ADID design tokens (colors, borders, radii, fonts) |
| `base.css` | (element styles) | HTML/body resets, grain overlay, scrollbar |
| `tailwind.css` | (theme block) | Tailwind v4 `@theme inline` mapping of ADI tokens |
| `animations.css` | `.stagger-1`..`.stagger-4` | Keyframes (fade-in, fade-in-up, slide-in-left, draw-line) + stagger delays |
| `glass.css` | `.glass`, `.glass-light` | Glass morphism backgrounds |
| `effects.css` | `.glow`, `.glow-sm`, `.gradient-text` | Box-shadow glow, gradient-filled text |
| `cards.css` | `.card-grid`, `.gradient-box`, `.card-hover` | Card patterns (gap-border grid, gradient border, hover line) |
| `layout.css` | `.section-separator`, `.container-narrow/content/wide` | Section spacing, container widths |
| `buttons.css` | `.btn`, `.btn-primary`, `.btn-secondary`, `.btn-sm`, `.btn-lg` | Button styles |
| `pills.css` | `.pill`, `.pill-muted/success/error/warning`, `.pill-group` | Pill/badge/tag patterns |
| `modal.css` | `.overlay-backdrop`, `.overlay-panel`, `.dropdown-panel`, `.dropdown-item` | Overlay and dropdown patterns |
| `prose.css` | `.prose` | Article typography (headings, lists, blockquotes, code, links) |
| `index.css` | (barrel) | Imports all snippets except `tailwind.css` and `tokens.css` |

## Usage

### Full import (with Tailwind + theme)
```css
@import "tailwindcss";
@import "../../packages/theme/generated/adi-theme.css";
@import "../../packages/css/tailwind.css";
@import "../../packages/css/index.css";
```

### Standalone (no theme package, no Tailwind)
```css
@import "../../packages/css/tokens.css";
@import "../../packages/css/index.css";
```

### Individual snippet import
```css
@import "../../packages/css/ax.css";
@import "../../packages/css/glass.css";
@import "../../packages/css/cards.css";
```

### AX system example
```html
<!-- Default spacing/text -->
<div class="p-v-1 p-h-2 text-base">Normal content</div>

<!-- Compact sidebar -->
<aside class="compact">
  <nav class="p-v-05 g-025 text-sm">Tighter spacing + smaller text</nav>
</aside>

<!-- Nested scaling: each .l-1/2 halves the inherited --l -->
<div class="l-2">
  <div class="p-v-1">32px padding</div>
  <div class="l-1/2">
    <div class="p-v-1">16px padding (halved)</div>
  </div>
</div>
```

## Dependencies
- `index.css` requires `packages/theme/generated/adi-theme.css` (provides `--adi-*` tokens)
- OR use `tokens.css` for standalone ADID tokens without the theme system
- `tailwind.css` requires Tailwind CSS v4
- `ax.css` is self-contained (uses `@property` registration)

## Adding New Snippets
1. Create `<name>.css` in this directory
2. Add import to `index.css`
3. Update the files table above
