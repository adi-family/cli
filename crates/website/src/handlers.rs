use axum::extract::{Path, State};
use axum::response::Html;

use crate::i18n::{register_tera_fn, resolve_translations};
use crate::lang::{Language, DEFAULT_LANG, SUPPORTED_LANGS};
use crate::AppState;

/// GET /
pub async fn home(State(state): State<AppState>) -> Html<String> {
    render(&state, DEFAULT_LANG, "home.html")
}

/// GET /{lang} or GET /{lang}/
pub async fn home_lang(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Html<String> {
    let lang = Language::from_code(&code).unwrap_or(DEFAULT_LANG);
    render(&state, lang, "home.html")
}

fn render(state: &AppState, lang: Language, template: &str) -> Html<String> {
    let mut tera = state.tera.clone();

    if state.mode == crate::Mode::Dev {
        if let Err(e) = tera.full_reload() {
            tracing::warn!("Template reload failed: {e}");
        }
    }

    let resolved = resolve_translations(&state.translations, lang);
    register_tera_fn(&mut tera, resolved);

    let mut ctx = tera::Context::new();
    ctx.insert("lang", lang.code());

    let langs: Vec<_> = SUPPORTED_LANGS
        .iter()
        .map(|l| {
            serde_json::json!({
                "code": l.code(),
                "native_name": l.native_name(),
                "english_name": l.english_name(),
                "is_active": *l == lang,
            })
        })
        .collect();
    ctx.insert("supported_langs", &langs);

    match tera.render(template, &ctx) {
        Ok(body) => Html(body),
        Err(e) => {
            tracing::error!("Template render error: {e}");
            Html(format!("<h1>Internal Server Error</h1><pre>{e}</pre>"))
        }
    }
}
