ui-components, lit, web-components, adid, ax-system, shared-library

- Shared Lit 3.x web component library published as `@adi-family/sdk-ui-components`
- All components use **light DOM** (`createRenderRoot() { return this; }`)
- Sizing via **ADID AX system** (`--l`, `--t`, `--r` CSS variables) -- no `size` prop
- Colors via **`--adi-*` design tokens** -- works with theme or standalone tokens
- Custom element tags prefixed with `adi-` to avoid collisions
- No Tailwind dependency -- uses inline styles referencing CSS custom properties

## ADID AX Sizing

Components do NOT have `sm`/`md`/`lg` props. Instead, sizing is **inherited from context**:

```html
<!-- Default size -->
<adi-primary-button label="Save"></adi-primary-button>

<!-- Compact (0.75x spacing, 0.875x text) -->
<div class="compact">
  <adi-primary-button label="Save"></adi-primary-button>
</div>

<!-- Dense (0.5x spacing, 0.75x text) -->
<nav class="dense">
  <adi-ghost-button label="Back"></adi-ghost-button>
</nav>

<!-- Spacious (1.5x spacing, 1.125x text) -->
<section class="spacious">
  <adi-text-input placeholder="Enter name..."></adi-text-input>
</section>
```

AX multipliers from `ax.css`:
- **Padding**: buttons use `0.75 * --l` block / `1.75 * --l` inline; inputs use `0.625 * --l` / `0.875 * --l`
- **Font size**: `calc(var(--t) * 0.875)` for body text
- **Border radius**: `var(--r)` cascades from parent (`.r-sm`, `.r-lg`, etc.)

## Components

### Buttons (`buttons/`)
| Tag | Class | Description |
|-----|-------|-------------|
| `<adi-primary-button>` | `AdiPrimaryButton` | Accent-tinted primary action |
| `<adi-secondary-button>` | `AdiSecondaryButton` | Border-only secondary action |
| `<adi-danger-button>` | `AdiDangerButton` | Error-tinted destructive action |
| `<adi-ghost-button>` | `AdiGhostButton` | Minimal, no border/background |
| `<adi-icon-button>` | `AdiIconButton` | Square icon-only, variants: default/primary/danger |
| `<adi-button-group>` | `AdiButtonGroup` | Segmented radio-group with keyboard nav |

All text buttons extend `BaseButton` which provides async `onClick` with auto-loading state.

### Inputs (`inputs/`)
| Tag | Class | Description |
|-----|-------|-------------|
| `<adi-text-input>` | `AdiTextInput` | Text field with label, error, clear, char count |
| `<adi-textarea-input>` | `AdiTextareaInput` | Multi-line with auto-resize, Cmd+Enter submit |
| `<adi-select-input>` | `AdiSelectInput` | Custom dropdown with keyboard nav + type-ahead |
| `<adi-search-input>` | `AdiSearchInput` | Search with debounce, loading spinner, "/" hint |
| `<adi-toggle-input>` | `AdiToggleInput` | Switch toggle with label |
| `<adi-checkbox-input>` | `AdiCheckboxInput` | Checkbox with indeterminate state |

### Feedback (`feedback/`)
| Tag | Class | Description |
|-----|-------|-------------|
| `<adi-loading-skeleton>` | `AdiLoadingSkeleton` | Shimmer placeholder (card/text/avatar variants) |

## Events
- Inputs emit `value-change` with the new value as `detail`
- Text/textarea/search emit `submit` on Enter/Cmd+Enter
- Toggle/checkbox emit `change` with boolean `detail`
- Button group emits `value-change` with selected option value

## Dependencies
- Requires `lit` ^3.3.1
- Requires ADID CSS variables (`--l`, `--t`, `--r`, `--adi-*`) from either:
  - `packages/theme/generated/adi-theme.css` + `packages/css/ax.css` (full theme)
  - `packages/css/tokens.css` + `packages/css/ax.css` (standalone)

## Consumption
```typescript
// Full import (registers all custom elements)
import "@adi-family/sdk-ui-components";

// Selective imports
import { AdiPrimaryButton } from "@adi-family/sdk-ui-components/buttons";
import { AdiTextInput } from "@adi-family/sdk-ui-components/inputs";
```

## Build
```bash
npm install
npm run build    # tsc -> dist/
npm run dev      # tsc --watch
```
