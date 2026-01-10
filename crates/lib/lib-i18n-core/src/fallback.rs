use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use std::collections::HashMap;

/// Implement fallback chain for translation lookup
///
/// Order: current_language → fallback_language (en-US) → raw key
pub fn get_message(
    bundles: &HashMap<String, FluentBundle<FluentResource>>,
    current_lang: &str,
    fallback_lang: &str,
    key: &str,
    args: Option<&HashMap<String, fluent_bundle::FluentValue>>,
) -> String {
    // Try current language first
    if let Some(message) = try_get_from_bundle(bundles, current_lang, key, args) {
        return message;
    }

    // Fallback to fallback language (typically en-US)
    if current_lang != fallback_lang {
        if let Some(message) = try_get_from_bundle(bundles, fallback_lang, key, args) {
            return message;
        }
    }

    // Last resort: return the key itself
    tracing::debug!("Translation key not found: {}", key);
    key.to_string()
}

/// Try to get a message from a specific language bundle
fn try_get_from_bundle(
    bundles: &HashMap<String, FluentBundle<FluentResource>>,
    lang: &str,
    key: &str,
    args: Option<&HashMap<String, fluent_bundle::FluentValue>>,
) -> Option<String> {
    let bundle = bundles.get(lang)?;

    let message = bundle.get_message(key)?;
    let pattern = message.value()?;

    let mut errors = vec![];
    let value = if let Some(args_map) = args {
        // Convert HashMap to FluentArgs
        let mut fluent_args = FluentArgs::new();
        for (k, v) in args_map {
            fluent_args.set(k.clone(), v.clone());
        }
        bundle.format_pattern(pattern, Some(&fluent_args), &mut errors)
    } else {
        bundle.format_pattern(pattern, None, &mut errors)
    };

    if !errors.is_empty() {
        tracing::warn!("Fluent formatting errors for key '{}': {:?}", key, errors);
    }

    Some(value.to_string())
}

/// Try to get a message attribute (e.g., "key.prefix")
pub fn get_attribute(
    bundles: &HashMap<String, FluentBundle<FluentResource>>,
    current_lang: &str,
    fallback_lang: &str,
    key: &str,
    attribute: &str,
) -> Option<String> {
    // Try current language first
    if let Some(message) = try_get_attribute_from_bundle(bundles, current_lang, key, attribute) {
        return Some(message);
    }

    // Fallback to fallback language
    if current_lang != fallback_lang {
        if let Some(message) = try_get_attribute_from_bundle(bundles, fallback_lang, key, attribute)
        {
            return Some(message);
        }
    }

    None
}

fn try_get_attribute_from_bundle(
    bundles: &HashMap<String, FluentBundle<FluentResource>>,
    lang: &str,
    key: &str,
    attribute: &str,
) -> Option<String> {
    let bundle = bundles.get(lang)?;
    let message = bundle.get_message(key)?;
    let attr = message.get_attribute(attribute)?;

    let mut errors = vec![];
    let value = bundle.format_pattern(attr.value(), None, &mut errors);

    if !errors.is_empty() {
        tracing::warn!(
            "Fluent formatting errors for attribute '{}.{}': {:?}",
            key,
            attribute,
            errors
        );
    }

    Some(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use unic_langid::LanguageIdentifier;

    #[test]
    fn test_fallback_chain() {
        let mut bundles = HashMap::new();

        // Create English bundle
        let en_ftl = "hello = Hello, { $name }!\ngoodbye = Goodbye!";
        let en_resource = FluentResource::try_new(en_ftl.to_string()).unwrap();
        let en_lang: LanguageIdentifier = "en-US".parse().unwrap();
        let mut en_bundle = FluentBundle::new(vec![en_lang]);
        en_bundle.add_resource(en_resource).unwrap();

        // Create Chinese bundle (incomplete)
        let zh_ftl = "hello = 你好, { $name }!";
        let zh_resource = FluentResource::try_new(zh_ftl.to_string()).unwrap();
        let zh_lang: LanguageIdentifier = "zh-CN".parse().unwrap();
        let mut zh_bundle = FluentBundle::new(vec![zh_lang]);
        zh_bundle.add_resource(zh_resource).unwrap();

        bundles.insert("en-US".to_string(), en_bundle);
        bundles.insert("zh-CN".to_string(), zh_bundle);

        // Test direct lookup (Chinese)
        let mut args = HashMap::new();
        args.insert(
            "name".to_string(),
            fluent_bundle::FluentValue::from("世界"),
        );
        let result = get_message(&bundles, "zh-CN", "en-US", "hello", Some(&args));
        assert!(result.contains("你好"));

        // Test fallback to English (key exists in en-US but not zh-CN)
        let result = get_message(&bundles, "zh-CN", "en-US", "goodbye", None);
        assert_eq!(result, "Goodbye!");

        // Test fallback to key (key doesn't exist anywhere)
        let result = get_message(&bundles, "zh-CN", "en-US", "missing-key", None);
        assert_eq!(result, "missing-key");
    }
}
