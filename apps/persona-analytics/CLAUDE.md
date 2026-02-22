persona-analytics, lit, vite, adid, css-sdk

- Lit web components app for persona analytics
- Uses `@adi-family/sdk-parts-css` for all styling (AX system, tokens, components)
- Vite dev server, TypeScript, no framework beyond Lit
- Proxied at `persona-analytics.adi.local/` via Hive (port 8051)

## Styling
- **No Shadow DOM** — all components must use `static shadowRootOptions` or `createRenderRoot() { return this; }` to render into light DOM
- Global CSS from the SDK applies directly to component markup
- Use AX system classes (`.l-*`, `.t-*`, `.r-*`, `.g-*`, `.p-*`, `.rounded-*`) for spacing/text/radius
- **Always use `--adi-*` token names** (`--adi-text`, `--adi-bg`, `--adi-border`, `--adi-accent`, etc.) — never raw names (`--white`, `--gray-*`, `--border`) — so light/dark mode and theme switching work correctly
- Co-locate component-specific styles in `<style>` tags or CSS files, not in Lit `static styles`
