mod handlers;
mod i18n;
pub mod lang;
pub mod tailwind;

use std::net::SocketAddr;
use std::path::PathBuf;

use axum::Router;
use i18n::Translations;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

/// Dev vs production run mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Dev,
    Prod,
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub tera: tera::Tera,
    pub translations: Translations,
    pub mode: Mode,
}

/// Build the axum router.
fn build_router(state: AppState, static_dir: PathBuf) -> Router {
    Router::new()
        .route("/", axum::routing::get(handlers::home))
        .route("/:lang", axum::routing::get(handlers::home_lang))
        .route("/:lang/", axum::routing::get(handlers::home_lang))
        .nest_service("/static", ServeDir::new(static_dir))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

/// Start the website server on the given port.
pub async fn run_server(port: u16, mode: Mode) -> anyhow::Result<()> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let templates_glob = crate_dir.join("templates/**/*");
    let static_dir = crate_dir.join("static");

    // Tailwind CSS — gracefully skip if binary not installed
    let _tw_child = match mode {
        Mode::Prod => {
            tailwind::build(&crate_dir)?;
            None
        }
        Mode::Dev => tailwind::watch(&crate_dir)?,
    };

    let tera = tera::Tera::new(
        templates_glob
            .to_str()
            .expect("templates path must be valid UTF-8"),
    )?;

    let translations = i18n::load_translations()?;

    let state = AppState { tera, translations, mode };
    let router = build_router(state, static_dir);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Website listening on {addr} ({mode:?})");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
