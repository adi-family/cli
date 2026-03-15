# Theme Respect

## Why This Rule Exists

ADI ships 10 themes with dark/light mode support. Users choose their theme, and every surface -- CLI, GUI, web -- must honor that choice. Hardcoded colors break the visual contract and make the product feel inconsistent.

## Source of Truth

`packages/theme/themes.json` defines all themes. The `sync-theme` workflow generates:

- `generated/themes.rs` -- included via `include!()` in Rust crates
- `generated/adi-theme.css` -- CSS custom properties for web apps

Status colors live in `packages/theme/status-colors.json` and are universal across all themes: success (`#22cc00`), error (`#ff0000`), warning (`#ffaa00`), info (follows accent).

## Theme Selection Priority

1. `ADI_THEME` environment variable
2. `theme` field in `config.toml`
3. Default: `indigo`

## Rules

### 1. Never Hardcode Colors

```rust
// BAD
println!("\x1b[32mSuccess\x1b[0m");
let color = Color::from_rgb(0.5, 0.2, 0.8);

// GOOD - CLI
out_success!("Operation completed");
theme::success("Success")

// GOOD - GUI
UiColors::from_theme(theme).accent
```

### 2. Use Semantic Color Functions

Map intent to color through the theme system, not through raw values:

| Intent | CLI (`theme::*`) | GUI (`UiColors`) |
|--------|-------------------|-------------------|
| Brand emphasis | `theme::brand(val)` | `colors.accent` |
| Bold brand | `theme::brand_bold(val)` | `colors.accent` + bold |
| Success | `theme::success(val)` | `colors.success` |
| Error | `theme::error(val)` | `colors.error` |
| Warning | `theme::warning(val)` | `colors.warning` |
| Info | `theme::info(val)` | `colors.info` |
| Muted/secondary | `theme::muted(val)` | `colors.text_muted` |
| Debug | `theme::debug(val)` | `colors.debug` |
| Default text | `theme::foreground(val)` | `colors.text` |

### 3. Web Apps Use CSS Custom Properties

```css
/* BAD */
.accent { color: #875fd7; }

/* GOOD */
.accent { color: var(--adi-accent); }
.bg { background: var(--adi-bg); }
.text { color: var(--adi-text); }
```

Available properties follow the token schema: `--adi-bg`, `--adi-surface`, `--adi-surface-alt`, `--adi-accent`, `--adi-accent-soft`, `--adi-text`, `--adi-text-muted`, `--adi-border`, `--adi-gradient`.

### 4. GUI Uses `UiColors::from_theme()`

All Iced components accept `UiColors` converted from the active theme. Never construct colors manually:

```rust
// BAD
let bg = Color::from_rgb(0.1, 0.1, 0.1);

// GOOD
let colors = UiColors::from_theme(&active_theme);
let bg = colors.bg;
```

### 5. Status Colors Are Universal

Success, error, warning, and info colors do not change between themes. They are defined once in `status-colors.json` and shared globally. Do not override them per-theme.

## The Test

*"Does this component look correct in every theme?"* If you used a hardcoded color, the answer is no.
