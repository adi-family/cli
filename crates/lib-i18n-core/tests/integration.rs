use lib_i18n_core::{I18n, ServiceDescriptor, ServiceHandle, ServiceRegistry, Result, I18nError};
use std::collections::HashMap;
use std::sync::Arc;

struct TestServiceRegistry {
    services: Vec<String>,
    messages: HashMap<String, String>,
    metadata: HashMap<String, String>,
}

impl ServiceRegistry for TestServiceRegistry {
    fn list_services(&self) -> Result<Vec<ServiceDescriptor>> {
        Ok(self
            .services
            .iter()
            .map(|id| ServiceDescriptor::new(id.clone()))
            .collect())
    }

    fn lookup_service(&self, service_id: &str) -> Result<Box<dyn ServiceHandle>> {
        if self.services.contains(&service_id.to_string()) {
            Ok(Box::new(TestServiceHandle {
                service_id: service_id.to_string(),
                messages: self.messages.clone(),
                metadata: self.metadata.clone(),
            }))
        } else {
            Err(I18nError::ServiceRegistryError(format!(
                "Service not found: {}",
                service_id
            )))
        }
    }
}

struct TestServiceHandle {
    service_id: String,
    messages: HashMap<String, String>,
    metadata: HashMap<String, String>,
}

impl ServiceHandle for TestServiceHandle {
    fn invoke(&self, method: &str, _args: &str) -> Result<String> {
        match method {
            "get_messages" => self
                .messages
                .get(&self.service_id)
                .cloned()
                .ok_or_else(|| I18nError::ServiceInvokeError("No messages".to_string())),
            "get_metadata" => self
                .metadata
                .get(&self.service_id)
                .cloned()
                .ok_or_else(|| I18nError::ServiceInvokeError("No metadata".to_string())),
            _ => Err(I18nError::ServiceInvokeError(format!(
                "Unknown method: {}",
                method
            ))),
        }
    }
}

#[test]
fn test_basic_translation() {
    let mut messages = HashMap::new();
    messages.insert(
        "adi.i18n.cli.en-US".to_string(),
        "hello = Hello, World!".to_string(),
    );

    let mut metadata = HashMap::new();
    metadata.insert(
        "adi.i18n.cli.en-US".to_string(),
        r#"{"plugin_id":"adi.cli","language":"en-US","language_name":"English","namespace":"cli","version":"1.0.0"}"#.to_string(),
    );

    let registry = Arc::new(TestServiceRegistry {
        services: vec!["adi.i18n.cli.en-US".to_string()],
        messages,
        metadata,
    });

    let mut i18n = I18n::new(registry).with_namespace("cli");
    i18n.discover_translations().unwrap();
    i18n.set_language("en-US").unwrap();

    assert_eq!(i18n.get("hello"), "Hello, World!");
}

#[test]
fn test_parametrized_messages() {
    let mut messages = HashMap::new();
    messages.insert(
        "adi.i18n.cli.en-US".to_string(),
        "greeting = Hello, { $name }!".to_string(),
    );

    let mut metadata = HashMap::new();
    metadata.insert(
        "adi.i18n.cli.en-US".to_string(),
        r#"{"plugin_id":"adi.cli","language":"en-US","language_name":"English","namespace":"cli","version":"1.0.0"}"#.to_string(),
    );

    let registry = Arc::new(TestServiceRegistry {
        services: vec!["adi.i18n.cli.en-US".to_string()],
        messages,
        metadata,
    });

    let mut i18n = I18n::new(registry).with_namespace("cli");
    i18n.discover_translations().unwrap();
    i18n.set_language("en-US").unwrap();

    let mut args = HashMap::new();
    args.insert("name".to_string(), lib_i18n_core::fluent_value_from("Alice"));
    let result = i18n.get_with_args("greeting", args);

    assert!(result.contains("Hello"));
    assert!(result.contains("Alice"));
}

#[test]
fn test_fallback_to_english() {
    let mut messages = HashMap::new();
    messages.insert(
        "adi.i18n.cli.en-US".to_string(),
        "hello = Hello!\ngoodbye = Goodbye!".to_string(),
    );
    messages.insert(
        "adi.i18n.cli.zh-CN".to_string(),
        "hello = 你好!".to_string(), // Only 'hello', no 'goodbye'
    );

    let mut metadata = HashMap::new();
    metadata.insert(
        "adi.i18n.cli.en-US".to_string(),
        r#"{"plugin_id":"adi.cli","language":"en-US","language_name":"English","namespace":"cli","version":"1.0.0"}"#.to_string(),
    );
    metadata.insert(
        "adi.i18n.cli.zh-CN".to_string(),
        r#"{"plugin_id":"adi.cli","language":"zh-CN","language_name":"Chinese","namespace":"cli","version":"1.0.0"}"#.to_string(),
    );

    let registry = Arc::new(TestServiceRegistry {
        services: vec![
            "adi.i18n.cli.en-US".to_string(),
            "adi.i18n.cli.zh-CN".to_string(),
        ],
        messages,
        metadata,
    });

    let mut i18n = I18n::new(registry).with_namespace("cli");
    i18n.discover_translations().unwrap();
    i18n.set_language("zh-CN").unwrap();

    // Should get Chinese version
    assert_eq!(i18n.get("hello"), "你好!");

    // Should fallback to English (not in Chinese bundle)
    assert_eq!(i18n.get("goodbye"), "Goodbye!");
}

#[test]
fn test_fallback_to_key() {
    let messages = HashMap::new();
    let metadata = HashMap::new();

    let registry = Arc::new(TestServiceRegistry {
        services: vec![],
        messages,
        metadata,
    });

    let mut i18n = I18n::new(registry).with_namespace("cli");
    i18n.discover_translations().unwrap();

    // Should fallback to key when no translations available
    assert_eq!(i18n.get("missing-key"), "missing-key");
}

#[test]
fn test_message_attributes() {
    let mut messages = HashMap::new();
    messages.insert(
        "adi.i18n.cli.en-US".to_string(),
        "status = Success!\n    .prefix = ✓".to_string(),
    );

    let mut metadata = HashMap::new();
    metadata.insert(
        "adi.i18n.cli.en-US".to_string(),
        r#"{"plugin_id":"adi.cli","language":"en-US","language_name":"English","namespace":"cli","version":"1.0.0"}"#.to_string(),
    );

    let registry = Arc::new(TestServiceRegistry {
        services: vec!["adi.i18n.cli.en-US".to_string()],
        messages,
        metadata,
    });

    let mut i18n = I18n::new(registry).with_namespace("cli");
    i18n.discover_translations().unwrap();
    i18n.set_language("en-US").unwrap();

    assert_eq!(i18n.get("status"), "Success!");
    assert_eq!(i18n.get("status.prefix"), "✓");
}

#[test]
fn test_available_languages() {
    let mut messages = HashMap::new();
    messages.insert(
        "adi.i18n.cli.en-US".to_string(),
        "hello = Hello!".to_string(),
    );
    messages.insert(
        "adi.i18n.cli.zh-CN".to_string(),
        "hello = 你好!".to_string(),
    );
    messages.insert(
        "adi.i18n.cli.uk-UA".to_string(),
        "hello = Привіт!".to_string(),
    );

    let mut metadata = HashMap::new();
    metadata.insert(
        "adi.i18n.cli.en-US".to_string(),
        r#"{"plugin_id":"adi.cli","language":"en-US","language_name":"English","namespace":"cli","version":"1.0.0"}"#.to_string(),
    );
    metadata.insert(
        "adi.i18n.cli.zh-CN".to_string(),
        r#"{"plugin_id":"adi.cli","language":"zh-CN","language_name":"Chinese","namespace":"cli","version":"1.0.0"}"#.to_string(),
    );
    metadata.insert(
        "adi.i18n.cli.uk-UA".to_string(),
        r#"{"plugin_id":"adi.cli","language":"uk-UA","language_name":"Ukrainian","namespace":"cli","version":"1.0.0"}"#.to_string(),
    );

    let registry = Arc::new(TestServiceRegistry {
        services: vec![
            "adi.i18n.cli.en-US".to_string(),
            "adi.i18n.cli.zh-CN".to_string(),
            "adi.i18n.cli.uk-UA".to_string(),
        ],
        messages,
        metadata,
    });

    let mut i18n = I18n::new(registry).with_namespace("cli");
    i18n.discover_translations().unwrap();

    let languages = i18n.available_languages();
    assert_eq!(languages.len(), 3);
    assert!(languages.contains(&"en-US".to_string()));
    assert!(languages.contains(&"zh-CN".to_string()));
    assert!(languages.contains(&"uk-UA".to_string()));
}
