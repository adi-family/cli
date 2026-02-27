use std::collections::HashMap;
use std::sync::Arc;

use lib_i18n_core::I18n;
use tera::{Function, Tera, Value};

use crate::lang::{Language, SUPPORTED_LANGS};

/// All known translation keys used in templates.
const TRANSLATION_KEYS: &[&str] = &[
    "site-name",
    "page-title",
    "nav-home",
    "nav-about",
    "footer-copyright",
    "home-title",
    "home-subtitle",
    "home-cta",
];

/// Pre-resolved translations: lang code → (key → translated string).
pub type Translations = HashMap<String, Arc<HashMap<String, String>>>;

/// Load all .ftl files and pre-resolve into flat string maps per language.
pub fn load_translations() -> anyhow::Result<Translations> {
    let mut map = Translations::new();

    for lang in SUPPORTED_LANGS {
        let code = lang.code();
        let mut i18n = I18n::new_standalone();

        let common = load_ftl(code, "common");
        let home = load_ftl(code, "home");
        let ftl = format!("{common}\n{home}");

        i18n.load_embedded(code, &ftl)?;
        i18n.set_language(code)?;

        let resolved: HashMap<String, String> = TRANSLATION_KEYS
            .iter()
            .map(|k| (k.to_string(), i18n.get(k)))
            .collect();

        map.insert(code.to_string(), Arc::new(resolved));
    }

    Ok(map)
}

/// Get the translation map for a language, falling back to English.
pub fn resolve_translations(translations: &Translations, lang: Language) -> &Arc<HashMap<String, String>> {
    translations
        .get(lang.code())
        .unwrap_or_else(|| translations.get("en").expect("English translations must exist"))
}

/// Register the `t(key="...")` Tera function backed by a pre-resolved map.
pub fn register_tera_fn(tera: &mut Tera, resolved: &Arc<HashMap<String, String>>) {
    tera.register_function("t", TranslateFn(Arc::clone(resolved)));
}

/// Tera function that resolves `{{ t(key="some-key") }}`.
#[derive(Clone)]
struct TranslateFn(Arc<HashMap<String, String>>);

impl Function for TranslateFn {
    fn call(&self, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let key = args
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| tera::Error::msg("t() requires a `key` argument"))?;

        let value = self
            .0
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_string());

        Ok(Value::String(value))
    }

    fn is_safe(&self) -> bool {
        true
    }
}

fn load_ftl(lang: &str, name: &str) -> String {
    match (lang, name) {
        ("en", "common") => include_str!("../locales/en/common.ftl").to_string(),
        ("en", "home") => include_str!("../locales/en/home.ftl").to_string(),
        ("uk", "common") => include_str!("../locales/uk/common.ftl").to_string(),
        ("uk", "home") => include_str!("../locales/uk/home.ftl").to_string(),
        ("hi", "common") => include_str!("../locales/hi/common.ftl").to_string(),
        ("hi", "home") => include_str!("../locales/hi/home.ftl").to_string(),
        ("ar", "common") => include_str!("../locales/ar/common.ftl").to_string(),
        ("ar", "home") => include_str!("../locales/ar/home.ftl").to_string(),
        ("pt", "common") => include_str!("../locales/pt/common.ftl").to_string(),
        ("pt", "home") => include_str!("../locales/pt/home.ftl").to_string(),
        ("zh", "common") => include_str!("../locales/zh/common.ftl").to_string(),
        ("zh", "home") => include_str!("../locales/zh/home.ftl").to_string(),
        ("ja", "common") => include_str!("../locales/ja/common.ftl").to_string(),
        ("ja", "home") => include_str!("../locales/ja/home.ftl").to_string(),
        _ => String::new(),
    }
}
