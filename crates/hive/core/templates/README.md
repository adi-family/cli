# Hive Error Page Templates

## Location
`crates/hive/core/templates/`

## Files
- `error_400.html` - Bad Request
- `error_404.html` - Not Found
- `error_502.html` - Bad Gateway (supports `{{logs}}` section)

## Design
- Uses ADI theme system with CSS custom properties
- Default: indigo dark (`--bg: #08080F`, `--accent: #6C5CE7`)
- Auto light mode via `prefers-color-scheme: light`
- Status colors: `--error: #FF4D6A` (400/502), `--warning: #E8A317` (404)
- Fonts: Space Grotesk (headings), Inter (body), JetBrains Mono (details)
- Gradient accent bar at top of card
- Centered card layout, responsive
- Embedded at compile time via `include_str!` in `error_pages.rs`

## Template Variables
All templates: `{{message}}`, `{{request_path}}`, `{{host}}`, `{{version}}`, `{{timestamp}}`
502 only: `{{service_name}}`, `{{logs}}` (pre-rendered HTML, not escaped)

## Module
`crates/hive/core/src/error_pages.rs`
- `not_found(message, path, host) -> Response`
- `bad_request(message, path, host) -> Response`
- `bad_gateway(message, path, host, service_name, logs) -> Response`
- `html_escape(s)` - XSS prevention for dynamic values
- `format_logs(logs)` - renders `LogLine[]` as colored HTML divs

## Config
In `hive.yaml`, enable log display on 502 pages:
```yaml
proxy:
  show_error_logs: true
```

## Testing

### Unit tests
```bash
cargo test -p hive-core error_pages
```

### Manual testing
```bash
# 1. Rebuild and install plugin
.adi/workflows/build-plugin.sh adi.hive --install --force --skip-lint

# 2. Restart hive daemon
adi hive daemon stop
adi hive up

# 3. Test 404 - hit a route that doesn't exist
curl http://adi.test/api/nonexistent

# 4. Test 502 - stop a service, then hit its route
adi hive stop default:auth
curl http://adi.test/api/auth/health

# 5. Test 502 with logs - add `show_error_logs: true` to proxy config first
```

### What to look for
- HTML response with styled card (not plain text)
- Content-Type: text/html
- Dynamic values (path, host) are HTML-escaped (no XSS)
- Gradient accent bar at top of card
- Dark mode by default, light mode auto-detected from OS
- 404 shows a hint box with guidance
- 502 page shows service name
- 502 page shows colored log lines when `show_error_logs: true`
- Log lines use ADI status colors (error=red, warn=yellow, info=text, debug=muted)
