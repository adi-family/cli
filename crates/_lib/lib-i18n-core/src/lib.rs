/*!
# lib-i18n-core

Internationalization library using Mozilla Fluent with plugin-based translation discovery.

## Features

- **Fluent-based**: Uses Mozilla's Fluent for industry-standard i18n
- **Plugin architecture**: Discovers translations from plugin services
- **Fallback chain**: Graceful degradation (lang → en-US → key)
- **Global macro**: Ergonomic `t!()` macro for translations
- **Thread-safe**: Global instance with concurrent access

## Usage

```rust,ignore
use lib_i18n_core::{I18n, init_global, t};
use std::sync::Arc;

// Initialize i18n with service registry from plugin host
let mut i18n = I18n::new(service_registry);
i18n.discover_translations()?;
i18n.set_language("zh-CN")?;

// Make globally available
init_global(i18n);

// Use translations anywhere in your code
println!("{}", t!("hello"));
println!("{}", t!("greeting", "name" => "Alice"));
```

## Translation Plugin Structure

Translation plugins register services with ID: `adi.i18n.{namespace}.{language}`

Example: `adi.i18n.cli.en-US`, `adi.i18n.tasks.zh-CN`

Each plugin provides two service methods:
- `get_messages()` → Returns .ftl file content
- `get_metadata()` → Returns JSON with language metadata

*/

mod core;
mod discovery;
mod error;
mod fallback;
mod localized;
pub mod r#macro;

// Re-export public API
pub use crate::core::I18n;
pub use crate::discovery::{
    ServiceDescriptor, ServiceHandle, ServiceRegistry, TranslationServiceInfo,
};
pub use crate::error::{I18nError, Result};
pub use crate::localized::LocalizedError;
pub use crate::r#macro::{fluent_value_from, global_instance, init_global, try_global_instance};

// Re-export fluent types for convenience
pub use fluent_bundle;

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _: fn(I18n) = init_global;
        let _: fn() -> Arc<Mutex<I18n>> = global_instance;
    }
}
