adi, styling, design-philosophy, theme, console-output, ui-patterns

## Design Philosophy

- **Sleek but pragmatic** — beauty serves function, never the reverse
- **Built for people by people** — every pixel, color, and animation exists because a human will feel something when they see it
- **Every element earns its place** — never add something just to fill space; if it doesn't help the user, remove it
- **Every action must be understandable** — the user should never wonder "what just happened?" or "what will this do?"
- **Readable by humans AND machines** — output must be parseable by AI tools and structured for automation (dual-mode: text + JSON stream)

### Emotional Design

Think about what users feel at each moment:

| Moment | Desired Emotion | How We Achieve It |
|--------|----------------|-------------------|
| First launch | Confidence, clarity | Clean output, no wall of text, immediate useful info |
| Waiting for operation | Calm, informed | Minimal spinner, clear status — not anxious loading bars |
| Success | Satisfaction, not overwhelm | Single check mark + result, not celebration fireworks |
| Error | Understanding, not panic | Clear what failed, why, and what to do next |
| Complex output | Mastery, orientation | Structured tables/cards with visual hierarchy |
| Idle / no data | Honesty | Say "nothing here" plainly, don't hide emptiness |

### Animation Rules

- **No animation is better than annoying animation** — default to static output
- Spinners: only for genuinely async operations (network, long compute)
- Progress bars: only when you know the total and it takes >2 seconds
- Live tables: only for dashboards/monitoring where data actually changes
- Never animate just to look cool — if removing the animation loses no information, remove it

## Theme System

- Source of truth: `packages/theme/themes.json` + `status-colors.json`
- Generated outputs (via `adi workflow sync-theme`): CSS (`adi-theme.css`) + Rust (`themes.rs`)
- 10 themes, each with dark + light mode
- Tokens per theme: `bg`, `surface`, `surfaceAlt`, `accent`, `accentSoft`, `text`, `textMuted`, `border`, `gradient`
- CLI theme priority: `ADI_THEME` env > `~/.config/adi/config.toml` > default `"indigo"`

### Status Colors (Universal)

| Color | Dark | Light | Use |
|-------|------|-------|-----|
| Success | #2ECC87 | #1A9960 | Completed, healthy, running |
| Error | #FF4D6A | #D63B52 | Failed, stopped, critical |
| Warning | #E8A317 | #B87F12 | Degraded, starting, attention |
| Info | theme accent | theme accent | Informational, brand-aligned |

### Web CSS Properties

```css
var(--adi-bg)  var(--adi-surface)  var(--adi-surfaceAlt)  var(--adi-accent)
var(--adi-accentSoft)  var(--adi-text)  var(--adi-textMuted)  var(--adi-border)
var(--adi-gradient)  var(--adi-success)  var(--adi-error)  var(--adi-warning)
```

Theme switching: `data-theme="scarlet"` + `data-mode="light"` on root element.

## CLI Output: `lib-console-output`

All CLI output MUST use `lib-console-output`. Never use raw `println!` with manual formatting.

### Theme Functions

```rust
use lib_console_output::theme;
theme::brand(val)       // Accent from active theme
theme::brand_bold(val)  // Accent + bold
theme::info(val)        // Brand-aligned info
theme::success(val)     // Green
theme::error(val)       // Red bold
theme::warning(val)     // Yellow
theme::debug(val)       // Cyan
theme::muted(val)       // Dim
theme::bold(val)        // Bold
```

### Icons

```rust
use lib_console_output::theme::icons;
icons::SUCCESS ✓  icons::ERROR ✕  icons::WARNING ⚠  icons::INFO ℹ
icons::DEBUG ›  icons::TRACE ·  icons::BRAND ◆  icons::PENDING ○  icons::IN_PROGRESS ◐
```

### Output Macros

```rust
out_trace!("...");  out_debug!("...");  out_info!("...");
out_success!("...");  out_warn!("...");  out_error!("...");
```

### Block Components

All implement `Renderable`: `.print() -> LiveHandle`, `.render() -> String`, `.line_count()`.

| Component | When to Use | Live Variant |
|-----------|------------|--------------|
| `Table` | Rows of data with borders (services, tasks) | `LiveTable` |
| `Columns` | Aligned data, no borders (summaries) | — |
| `Card` | Single-entity detail panel | — |
| `KeyValue` | Label-value pairs (config, properties) | `LiveKeyValue` |
| `Section` | Header separator (`── Title ──`) | — |
| `List` | Bullet or numbered items | — |

### Progress

```rust
use lib_console_output::progress::*;
spinner("Loading...");              // Only for async >1s
progress_bar(total, "Downloading"); // Only when total is known
steps(count, "Setup");              // Multi-step sequential
```

### Input

```rust
use lib_console_output::input::*;
Select::new("Choose").option(SelectOption::new("A", "a")).run();
MultiSelect::new("Pick").option(SelectOption::new("X", "x")).run();
Confirm::new("Continue?").default(true).run();
Input::new("Name").default("Anonymous").run();
Password::new("Secret").confirm("Confirm").run();
```

## Visual Hierarchy

### Color Usage

- **Brand/accent**: titles, headers, highlighted values, interactive elements
- **Success green**: only genuinely positive states (running, completed, healthy)
- **Error red**: only actual failures, not "stopped by user"
- **Warning yellow**: transitional states, degraded, needs attention
- **Muted/dim**: borders, separators, secondary info, timestamps, IDs
- **Bold**: prompts needing attention, section titles

### Spacing

- One blank line between logical sections
- No blank line inside a component
- Section headers get one blank line after

### What NOT to Do

- No ASCII art banners or logos in CLI output
- No color for color's sake — every color must convey meaning
- No nested tables or cards — keep structure flat
- No walls of unstructured text — always use a component
- No "pretty" formatting that breaks when piped
- No emojis in CLI output (use `theme::icons` instead)

## Web UI Patterns

- Use CSS custom properties (`var(--adi-*)`) for all colors — never hardcode hex
- Transitions: `150ms ease` hover, `200ms ease` layout
- Border radius: `8px` cards, `4px` inputs, `12px` modals
- Headings: `var(--adi-font-heading)`, body: `var(--adi-font-body)`, code: `var(--adi-font-mono-body)`
- Mobile-first; breakpoints: 640/768/1024/1280px

## Before Shipping UI

- Every color from theme tokens?
- Every CLI output uses `lib-console-output`?
- Output understandable with zero context?
- User knows what happened and what to do next?
- Any animation removable without losing information?
- Works in dark + light mode?
- Works when piped (`| less`, `> file.txt`)?
