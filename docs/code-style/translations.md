# Translations Convention

**Use Mozilla Fluent (.ftl) for all user-facing strings.**

## Structure

Translations live in `langs/` subdirectory within each crate that needs i18n:

```
crates/<component>/
  langs/
    build.rs          # Generates translation modules
    translation.rs    # Translation trait/helpers
    en/               # English (primary)
      Cargo.toml
      messages.ftl
    de/               # German
      Cargo.toml
      messages.ftl
    ...
```

Or for the main CLI in `crates/cli/plugins/`:

```
crates/cli/plugins/
  build.rs
  translation.rs
  en-US/messages.ftl
  de-DE/messages.ftl
  es-ES/messages.ftl
  fr-FR/messages.ftl
  ja-JP/messages.ftl
  ko-KR/messages.ftl
  ru-RU/messages.ftl
  uk-UA/messages.ftl
  zh-CN/messages.ftl
```

## Fluent Format

```ftl
# Domain header comment
# ============================================================================
# DOMAIN NAME
# ============================================================================

message-key = Simple message
message-with-var = Hello, { $name }!
message-with-plural = { $count ->
    [one] { $count } item
   *[other] { $count } items
}
```

## Rules

1. **English first** - always write `en` or `en-US` messages first
2. **Key naming** - use `domain-action-detail` format (e.g., `auth-login-success`)
3. **Variables** - use `{ $varName }` syntax
4. **Plurals** - use Fluent's select expressions for pluralization
5. **No hardcoded strings** - all user-visible text must use i18n

## Adding New Language

1. Create directory: `langs/<lang-code>/` or `plugins/<lang-code>/`
2. Add `Cargo.toml` with translation plugin metadata
3. Copy `en/messages.ftl` and translate
4. Build system auto-discovers new languages via `build.rs`
