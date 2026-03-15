# GUI Components

## Why This Rule Exists

`lib-iced-ui` provides pre-built, theme-aware components for all GUI surfaces. Using them ensures visual consistency, accessibility, and automatic theme adaptation without duplicating styling logic.

## Component Catalog

### Buttons

| Function | Use Case |
|----------|----------|
| `primary_button` | Main actions (submit, confirm) |
| `secondary_button` | Alternative actions |
| `text_button` | Minimal emphasis (links, tertiary) |
| `icon_button` | Icon-only actions |
| `pill_button` | Compact toggle/filter actions |
| `tab_button` | Tab navigation |
| `close_button` | Dismiss/close |
| `action_button` | Contextual actions in lists |
| `header_button` | Top bar actions |

### Cards and Containers

| Function | Use Case |
|----------|----------|
| `card` | General content panel (with `CardStyle` variants) |
| `modal_card` | Modal dialog content |
| `backdrop` | Overlay background dimming |
| `section` | Grouped content region |
| `code_block` | Code/output display |

`CardStyle` variants: `Default`, `Running`, `Success`, `Error`, `Interactive`, `System`.

### Inputs

| Function | Use Case |
|----------|----------|
| `command_input` | Command entry (monospace, `CODE_FONT`) |
| `search_input` | Search fields (sans-serif) |
| `styled_input` | Configurable general input |

### Status Indicators

| Function | Use Case |
|----------|----------|
| `status_pill` | General status badge |
| `git_pill` | Git branch/status |
| `running_pill` | Active process indicator |
| `env_pill` | Environment label |
| `stats_pill` | Numeric metric display |

### Tabs

| Function | Use Case |
|----------|----------|
| `session_tabs` | Session/workspace switching |
| `simple_tabs` | Content section switching |
| `nav_tabs` | Navigation menu |

## Rules

### 1. Always Pass `UiColors`

Every component takes `UiColors` as a parameter. Convert from the active theme once and pass through:

```rust
let colors = UiColors::from_theme(&theme);
primary_button(&colors, "Save")
```

### 2. Use the Right Component for the Job

Don't build custom buttons when `primary_button` / `secondary_button` exist. Don't hand-style cards when `card` with `CardStyle` covers the case.

### 3. Match Card Style to Semantic Meaning

```rust
// Process currently running
card(&colors, CardStyle::Running, content)

// Completed successfully
card(&colors, CardStyle::Success, content)

// Error state
card(&colors, CardStyle::Error, content)
```

### 4. Monospace for Code, Sans-serif for UI

`command_input` uses `CODE_FONT` (JetBrains Mono). `search_input` uses sans-serif (Inter). Don't mix them.

### 5. Compose, Don't Recreate

Build screens from existing components. If a new component is needed, add it to `lib-iced-ui` rather than styling inline in the app crate.

## The Test

*"Am I styling this element manually instead of using a lib-iced-ui component?"* If yes, check the catalog first.
