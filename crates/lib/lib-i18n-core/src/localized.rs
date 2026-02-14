use crate::r#macro::try_global_instance;
use fluent_bundle::FluentValue;
use std::collections::HashMap;

/// Trait for errors that carry an i18n slug key and can produce a localized message.
///
/// Implementors provide `slug()` (the Fluent message ID) and `i18n_args()`
/// (a map of placeholder names to values). The default `localized()` method
/// resolves these through the global I18n instance, falling back to the raw
/// slug if the i18n system is not initialized.
pub trait LocalizedError: std::error::Error {
    /// The Fluent message key for this error (e.g., "error-component-not-found").
    fn slug(&self) -> &str;

    /// Arguments to interpolate into the Fluent message.
    fn i18n_args(&self) -> HashMap<String, FluentValue<'static>> {
        HashMap::new()
    }

    /// Produce the localized error message.
    ///
    /// Falls back to the raw slug if the i18n system is not initialized
    /// or if the key is missing from all bundles.
    fn localized(&self) -> String {
        let slug = self.slug();
        let args = self.i18n_args();

        match try_global_instance() {
            Some(i18n) => {
                let guard = i18n.lock();
                if args.is_empty() {
                    guard.get(slug)
                } else {
                    guard.get_with_args(slug, args)
                }
            }
            None => slug.to_string(),
        }
    }
}
