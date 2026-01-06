use crate::core::I18n;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::sync::Arc;

/// Global I18n instance wrapped in Arc<Mutex> for thread-safety
static GLOBAL_I18N: OnceCell<Arc<Mutex<I18n>>> = OnceCell::new();

/// Initialize the global I18n instance
///
/// This should be called once at application startup after setting up
/// the plugin runtime and discovering translations.
///
/// # Panics
/// Panics if called more than once
pub fn init_global(i18n: I18n) {
    GLOBAL_I18N
        .set(Arc::new(Mutex::new(i18n)))
        .expect("Global I18n already initialized");
}

/// Get a reference to the global I18n instance
///
/// # Panics
/// Panics if `init_global()` was not called first
pub fn global_instance() -> Arc<Mutex<I18n>> {
    GLOBAL_I18N
        .get()
        .expect("Global I18n not initialized. Call init_global() first.")
        .clone()
}

/// Translation macro for simple message lookups
///
/// # Examples
///
/// ```ignore
/// // Simple message
/// let msg = t!("hello");
///
/// // Message with arguments
/// let msg = t!("greeting", "name" => "Alice");
///
/// // Message with multiple arguments
/// let msg = t!("complex", "count" => 42, "name" => "Bob");
///
/// // Message attribute (e.g., "hello.prefix")
/// let prefix = t!("hello.prefix");
/// ```
#[macro_export]
macro_rules! t {
    // Simple key lookup
    ($key:expr) => {{
        $crate::global_instance().lock().get($key)
    }};

    // Key with single argument
    ($key:expr, $arg_key:expr => $arg_val:expr) => {{
        let mut args = ::std::collections::HashMap::new();
        args.insert(
            $arg_key.to_string(),
            $crate::fluent_value_from($arg_val),
        );
        $crate::global_instance().lock().get_with_args($key, args)
    }};

    // Key with multiple arguments
    ($key:expr, $($arg_key:expr => $arg_val:expr),+ $(,)?) => {{
        let mut args = ::std::collections::HashMap::new();
        $(
            args.insert(
                $arg_key.to_string(),
                $crate::fluent_value_from($arg_val),
            );
        )+
        $crate::global_instance().lock().get_with_args($key, args)
    }};
}

/// Helper to convert values to FluentValue
///
/// This is public but hidden from docs as it's only used by the t!() macro
#[doc(hidden)]
pub fn fluent_value_from<T: Into<String>>(value: T) -> fluent_bundle::FluentValue<'static> {
    fluent_bundle::FluentValue::String(value.into().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::ServiceRegistry;
    use std::sync::Arc;

    // Mock implementation for testing
    struct MockRegistry;
    impl ServiceRegistry for MockRegistry {
        fn list_services(
            &self,
        ) -> crate::error::Result<Vec<crate::discovery::ServiceDescriptor>> {
            Ok(vec![])
        }
        fn lookup_service(
            &self,
            _service_id: &str,
        ) -> crate::error::Result<Box<dyn crate::discovery::ServiceHandle>> {
            Err(crate::error::I18nError::ServiceRegistryError(
                "Mock".to_string(),
            ))
        }
    }

    #[test]
    fn test_fluent_value_conversion() {
        let value = fluent_value_from("test");
        assert!(matches!(value, fluent_bundle::FluentValue::String(_)));

        let value = fluent_value_from(String::from("test"));
        assert!(matches!(value, fluent_bundle::FluentValue::String(_)));
    }

    #[test]
    #[should_panic(expected = "Global I18n already initialized")]
    fn test_double_init_panics() {
        // Create two instances
        let registry1 = Arc::new(MockRegistry);
        let i18n1 = I18n::new(registry1);

        let registry2 = Arc::new(MockRegistry);
        let i18n2 = I18n::new(registry2);

        // This should work
        init_global(i18n1);

        // This should panic
        init_global(i18n2);
    }
}
