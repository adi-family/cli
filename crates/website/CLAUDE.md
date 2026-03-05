website, axum, tera, tailwind, adid, i18n, ssr

## Stack
- **Rust**: Axum web server with Tera templates (SSR)
- **CSS**: Tailwind v4 + ADID design system (theme tokens, Tailwind utilities, component snippets)
- **i18n**: Mozilla Fluent (.ftl) via `lib-i18n-core`, embedded at compile time
- **Binary**: Self-contained — templates, translations, and static assets baked into the binary

## Running
- Dev (template hot-reload + Tailwind watch): `cargo run -p website -- dev`
- Prod (minified CSS, no reload): `cargo run -p website`
- Default port: 3080, override with `PORT` env var
- Requires `tailwindcss` CLI in PATH (gracefully skips if missing)

## File Structure
- `src/lib.rs` — Router, AppState, server startup
- `src/handlers.rs` — Route handlers, template rendering (auto-reloads in dev mode)
- `src/i18n.rs` — Translation loading, `t(key="...")` Tera function
- `src/lang.rs` — `Language` enum (En, Uk), `from_code()` parser
- `src/tailwind.rs` — Tailwind CLI build/watch helpers
- `templates/base.html` — HTML shell (nav, footer, fonts, `{% block content %}`)
- `templates/home.html` — Home page content (extends base.html)
- `static/input.css` — Tailwind entry point with ADID imports
- `static/style.css` — Generated (gitignored)
- `static/main.js` — Client-side entry point (placeholder)
- `locales/{en,uk}/` — Fluent translation files (common.ftl, home.ftl)

## CSS Architecture
- `static/input.css` imports: tailwindcss, adi-theme.css (tokens), tailwind.css (theme mapping), index.css (components)
- Use `@source "../templates/**/*.html"` for Tailwind class scanning
- Tailwind utilities for layout: `flex`, `items-center`, `ml-auto`, `h-[90vh]`
- ADID tokens via Tailwind: `bg-bg`, `bg-surface`, `text-text`, `text-text-muted`, `text-accent`, `border-border`, `font-heading`, `font-body`
- Tailwind utilities for spacing/sizing: `gap-6`, `py-4`, `px-8`, `space-y-3`, `text-sm`, `text-lg`, `text-3xl`
- ADID component classes: `btn btn-primary`, `pill-group`, `gradient-text`, `section-separator`, `container-content`

## Icons (Lucide)
- Loaded via CDN (`unpkg.com/lucide@latest`), initialized in `main.js`
- Usage in templates: `<i data-lucide="icon-name"></i>`
- Browse icons at https://lucide.dev/icons
- Sizing via Tailwind: `<i data-lucide="heart" class="w-5 h-5"></i>`
- Stroke color inherits from `currentColor` (use `text-*` classes)

## Templates (Tera)
- Translations: `{{ t(key="some-key") }}`
- Language variable: `{{ lang }}` (e.g. "en", "uk")
- Conditionals: `{% if lang == 'en' %}...{% endif %}`
- Blocks: `{% block content %}{% endblock content %}`
- Base template provides: nav, footer, font imports, ADID dark mode

## Adding a New Page
1. Create `templates/<page>.html` extending `base.html`
2. Add route in `src/lib.rs` (`build_router`)
3. Add handler in `src/handlers.rs` calling `render()`

## Adding Translations
1. Add key to `TRANSLATION_KEYS` in `src/i18n.rs`
2. Add entries to `locales/en/<file>.ftl` and `locales/uk/<file>.ftl`
3. For a new .ftl file: add `include_str!()` match arm in `load_ftl()` and concat in `load_translations()`

## Adding a New Language
1. Add variant to `Language` enum in `src/lang.rs`
2. Add to `SUPPORTED_LANGS` array
3. Add `from_code()` match arm
4. Create `locales/<code>/` directory with .ftl files
5. Add `include_str!()` match arms in `src/i18n.rs`
