ui-components, lit, web-components, adid, shared-library

- Shared Lit 3.x web component library published as `@adi-family/sdk-ui-components`
- All components use **light DOM** (`createRenderRoot() { return this; }`)
- Uses fixed rem-based sizing
- Colors via **`--adi-*` design tokens** -- works with theme or standalone tokens
- Custom element tags prefixed with `adi-` to avoid collisions
- No Tailwind dependency -- uses inline styles referencing CSS custom properties

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
- Requires `--adi-*` CSS variables from `packages/theme/generated/adi-theme.css` or `packages/css/tokens.css`

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
