css, design-system, adid, components, snippets, reusable

## Overview
- Reusable CSS component snippets for all ADI websites
- Implements the ADID design system
- Pure CSS classes using `--adi-*` design tokens from `packages/theme`
- No framework dependency -- works with any stack that imports the theme CSS
- Each file is a standalone snippet; import individually or via `index.css` barrel

## Files

| File | Classes | Purpose |
|------|---------|---------|
| `tokens.css` | (custom properties) | Standalone ADID design tokens (colors, borders, radii, fonts) |
| `base.css` | (element styles) | HTML/body resets, grain overlay, scrollbar |
| `tailwind.css` | (theme block) | Tailwind v4 `@theme inline` mapping of ADI tokens |
| `plugin-base.css` | (imports) | Bundle base for web plugins — imports theme + tailwind mapping |
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
@import "../../packages/css/glass.css";
@import "../../packages/css/cards.css";
```

## Dependencies
- `index.css` requires `packages/theme/generated/adi-theme.css` (provides `--adi-*` tokens)
- OR use `tokens.css` for standalone ADID tokens without the theme system
- `tailwind.css` requires Tailwind CSS v4

## Adding New Snippets
1. Create `<name>.css` in this directory
2. Add import to `index.css`
3. Update the files table above
