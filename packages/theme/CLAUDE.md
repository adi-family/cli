theme, design-system, colors, fonts, css, rust, unified

## Overview
- Single source of truth for all ADI visual identity across CLI, web apps, and extensions
- 10 themes with dark/light mode, per-theme fonts, universal status colors
- JSON source files synced to CSS custom properties and Rust constants via `sync-theme` workflow
- Default theme: `indigo`

## File Structure
- `themes.json` — source of truth: 10 themes (colors + fonts per theme)
- `status-colors.json` — universal status colors (success/error/warning/info) for dark/light
- `generated/adi-theme.css` — generated CSS custom properties (all themes, dark/light/auto)
- `generated/themes.rs` — generated Rust structs and constants

## Available Themes

| ID | Name | Accent | Heading Font |
|----|------|--------|--------------|
| `indigo` | Indigo & Ivory | `#6C5CE7` | Space Grotesk |
| `scarlet` | Scarlet Night | `#E74C3C` | Space Grotesk |
| `emerald` | Emerald Depths | `#2ECC71` | Plus Jakarta Sans |
| `teal` | Teal Horizon | `#1ABC9C` | DM Sans |
| `amber` | Amber Glow | `#F39C12` | Outfit |
| `neonRed` | Neon Red | `#FF3B3B` | Sora |
| `electricBlue` | Electric Blue | `#0984E3` | Urbanist |
| `hotPink` | Hot Pink | `#FD79A8` | Poppins |
| `acidLime` | Acid Lime | `#A8E617` | Manrope |
| `coralNavy` | Coral & Navy | `#FF7675` | Lexend |

## Token Schema (per theme, per mode)

| Token | CSS Variable | Description |
|-------|-------------|-------------|
| `bg` | `--adi-bg` | Page background |
| `surface` | `--adi-surface` | Card/panel background |
| `surfaceAlt` | `--adi-surface-alt` | Alternate surface (hover, sidebar) |
| `accent` | `--adi-accent` | Primary accent color |
| `accentSoft` | `--adi-accent-soft` | Accent with low opacity (highlights) |
| `text` | `--adi-text` | Primary text / headings |
| `textMuted` | `--adi-text-muted` | Secondary/muted text |
| `border` | `--adi-border` | Borders and dividers |
| `gradient` | `--adi-gradient` | Brand gradient |

## Font Tokens (per theme)

| Token | CSS Variable | Description |
|-------|-------------|-------------|
| `fontHeading` | `--adi-font-heading` | Headings, titles, hero text |
| `fontBody` | `--adi-font-body` | Body text, paragraphs, UI labels |
| `fontMonoHeading` | `--adi-font-mono-heading` | Code block headings, terminal titles |
| `fontMonoBody` | `--adi-font-mono-body` | Inline code, terminal body text |

## Status Colors (universal, not per-theme)

| Token | Dark | Light | Description |
|-------|------|-------|-------------|
| `success` | `#2ECC87` | `#1A9E62` | Success states |
| `error` | `#FF4D6A` | `#D03040` | Error states |
| `warning` | `#E8A317` | `#B87D0A` | Warning states |
| `info` | `var(--adi-accent)` | `var(--adi-accent)` | Info (follows accent) |

## Regenerating Outputs

```bash
adi workflow sync-theme
# or directly:
.adi/workflows/sync-theme.sh
```

Reads `themes.json` + `status-colors.json`, generates:
- `generated/adi-theme.css` — all themes as CSS custom properties
- `generated/themes.rs` — Rust `Theme`/`ThemeMode`/`ThemeFonts` structs + `THEMES` array

## Usage: Web Apps (CSS)

Import the generated CSS and map to Tailwind utilities:

```css
@import "tailwindcss";
@import "../../../../packages/theme/generated/adi-theme.css";

@theme inline {
  --color-bg: var(--adi-bg);
  --color-surface: var(--adi-surface);
  --color-accent: var(--adi-accent);
  --color-text: var(--adi-text);
  --color-text-muted: var(--adi-text-muted);
  --color-border: var(--adi-border);
  --color-success: var(--adi-success);
  --color-error: var(--adi-error);
  --color-warning: var(--adi-warning);

  --font-heading: var(--adi-font-heading);
  --font-body: var(--adi-font-body);
  --font-mono-heading: var(--adi-font-mono-heading);
  --font-mono-body: var(--adi-font-mono-body);
}
```

Tailwind classes: `bg-bg`, `bg-surface`, `text-text`, `text-accent`, `border-border`, `text-success`, `bg-error-soft`, `font-heading`, `font-mono-body`, etc.

## Usage: Dark/Light Mode Switching

```html
<!-- Default: indigo dark (no attributes needed) -->
<html>

<!-- Explicit dark -->
<html data-mode="dark">

<!-- Explicit light -->
<html data-mode="light">

<!-- Auto from OS preference (default behavior when no data-mode) -->

<!-- Theme selection -->
<html data-theme="scarlet">

<!-- Theme + light mode -->
<html data-theme="scarlet" data-mode="light">
```

## Usage: Rust CLI

The `lib-console-output` crate includes the generated themes via `theme::generated::*`.

```rust
use lib_console_output::theme;

// Initialize from env var or config (call early in main)
// Reads ADI_THEME env var, falls back to default ("indigo")
theme::init("scarlet");

// Brand/accent color follows the active theme
println!("{}", theme::brand("Hello"));       // Scarlet accent
println!("{}", theme::brand_bold("Title"));  // Scarlet accent, bold

// Status colors are universal (same across all themes)
println!("{}", theme::success("OK"));    // Green
println!("{}", theme::error("Fail"));    // Red bold
println!("{}", theme::warning("Warn"));  // Yellow

// Access full theme data
let t = theme::active();
println!("Theme: {} ({})", t.name, t.id);
println!("Accent: {}", t.accent);
println!("Dark bg: {}", t.dark.bg);
println!("Heading font: {}", t.fonts.heading);

// Find a specific theme
if let Some(emerald) = theme::find_theme("emerald") {
    println!("Emerald accent: {}", emerald.accent);
}

// List all themes
for t in theme::generated::THEMES {
    println!("{}: {}", t.id, t.name);
}
```

### CLI Theme Selection Priority
1. `ADI_THEME` environment variable (highest)
2. `theme` field in `~/.config/adi/config.toml`
3. Default: `"indigo"`

```bash
# Override via env var
ADI_THEME=scarlet adi tasks list

# Persist in config
# ~/.config/adi/config.toml
# theme = "scarlet"
```

## Adding a New Theme

1. Add theme entry to `themes.json` under `themes` key (dark + light modes)
2. Add font entry to `themes.json` under `fonts` key
3. Run `adi workflow sync-theme`
4. Commit all generated files

## Modifying Status Colors

Edit `status-colors.json` (dark/light variants), then run `adi workflow sync-theme`.

## Architecture

```
packages/theme/themes.json          ← edit here
packages/theme/status-colors.json   ← edit here
         │
         ▼
.adi/workflows/sync-theme.sh        ← generates
         │
         ├──▶ packages/theme/generated/adi-theme.css  → web apps (CSS import)
         └──▶ packages/theme/generated/themes.rs      → lib-console-output (include!)
                                                          → CLI (theme::init)
```
