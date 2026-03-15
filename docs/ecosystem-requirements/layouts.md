# Layouts

## Why This Rule Exists

Consistent spacing, sizing, and typography across all GUI surfaces prevents the product from feeling like a patchwork of disconnected screens. `lib-terminal-theme` defines layout constants consumed by `lib-iced-ui` -- use them instead of magic numbers.

## Layout Tokens

Source: `lib-terminal-theme` `LayoutConfig`

| Token | Default | Purpose |
|-------|---------|---------|
| `content_padding` | 40 | Main content area padding |
| `bar_padding` | 6 | Toolbar/bar internal padding |
| `element_spacing` | 12 | Gap between sibling elements |
| `small_spacing` | 4 | Compact gap (pills, inline items) |
| `border_radius` | 6 | Standard corner radius |
| `pill_radius` | 12 | Pill/badge corner radius |
| `overlay_radius` | 12 | Modal/overlay corner radius |
| `header_height` | 33 | Top bar height |
| `sidebar_width` | 200 | Sidebar panel width |
| `scrollbar_width` | 8 | Scrollbar track width |
| `palette_width` | 500 | Command palette width |
| `button_padding_h` | 12 | Button horizontal padding |
| `button_padding_v` | 6 | Button vertical padding |
| `input_padding_h` | 10 | Input horizontal padding |
| `input_padding_v` | 8 | Input vertical padding |
| `icon_padding` | 6 | Icon button padding |

## Typography Tokens

Source: `lib-terminal-theme` `Typography`

| Token | Default | Purpose |
|-------|---------|---------|
| `command_size` | 16 | Command input text |
| `output_size` | 14 | Terminal/output text |
| `hint_size` | 12 | Hints, secondary labels |
| `label_size` | 14 | Form labels, standard text |
| `header_size` | 20 | Section headers |

## Font System

| Context | Sans-serif | Monospace | Icons |
|---------|------------|-----------|-------|
| GUI (Iced) | Inter | JetBrains Mono | Phosphor |
| CLI | terminal default | terminal default | Unicode symbols |
| Web | theme fonts (CSS vars) | theme monospace (CSS vars) | -- |

## Animation Tokens

Source: `lib-terminal-theme` `AnimationConfig`

| Token | Default | Purpose |
|-------|---------|---------|
| `transition` | 200ms | Standard state transition |
| `animation_tick` | 16ms | Frame interval (~60fps) |
| `toast_fade` | 150ms | Toast notification fade |
| `hover_opacity` | 0.08 | Hover state overlay |
| `active_opacity` | 0.15 | Active/pressed overlay |
| `disabled_opacity` | 0.5 | Disabled element opacity |

## Rules

### 1. Never Use Magic Numbers for Spacing

```rust
// BAD
let padding = Padding::new(15.0);
let spacing = 8.0;

// GOOD
let padding = Padding::new(colors.content_padding);
let spacing = colors.element_spacing;
```

### 2. Use Typography Tokens for Font Sizes

```rust
// BAD
let size = 13.0;

// GOOD
let size = colors.output_size;
```

### 3. Respect the Animation Timing

```rust
// BAD
Duration::from_millis(300)

// GOOD
Duration::from_millis(colors.transition_ms as u64)
```

### 4. Overlay Shadows Use Defined Tiers

Three shadow levels: small, medium, large. Each has predefined offset, blur, and color. Don't invent shadow values.

### 5. Web Apps Follow the Same Tokens

CSS variables mirror Rust constants. Use `var(--adi-spacing-*)`, `var(--adi-radius-*)`, etc. where available, or match the token values from the table above.

## The Test

*"Did I use a literal number for spacing, sizing, or timing?"* If yes, replace it with a layout/typography/animation token.
