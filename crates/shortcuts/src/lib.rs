use lib_plugin_prelude::*;

use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

static SHORTCUTS: OnceLock<Arc<HashMap<String, String>>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct Config {
    shortcuts: HashMap<String, String>,
}

#[derive(Serialize)]
struct ShortcutEntry {
    name: String,
    url: String,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

pub struct ShortcutsPlugin;

#[async_trait]
impl Plugin for ShortcutsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.shortcuts", "Shortcuts", env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Core)
            .with_author("ADI Team")
            .with_description("URL redirect service for ADI shortcut URLs")
    }

    async fn init(&mut self, ctx: &PluginContext) -> Result<()> {
        PluginCtx::init(ctx);

        let path = PluginCtx::config_dir().join("shortcuts.yaml");
        let shortcuts = if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| PluginError::InitFailed(format!("failed to read {}: {e}", path.display())))?;
            let config: Config = serde_yml::from_str(&content)
                .map_err(|e| PluginError::InitFailed(format!("failed to parse {}: {e}", path.display())))?;
            config.shortcuts
        } else {
            HashMap::new()
        };

        let _ = SHORTCUTS.set(Arc::new(shortcuts));
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_HTTP_ROUTES]
    }
}

fn shortcuts() -> &'static Arc<HashMap<String, String>> {
    SHORTCUTS.get().expect("shortcuts not initialized")
}

#[async_trait]
impl HttpRoutes for ShortcutsPlugin {
    async fn list_routes(&self) -> Vec<HttpRoute> {
        vec![
            HttpRoute {
                method: HttpMethod::Get,
                path: "/health".to_string(),
                handler_id: "health".to_string(),
                description: "Health check".to_string(),
            },
            HttpRoute {
                method: HttpMethod::Get,
                path: "/".to_string(),
                handler_id: "list".to_string(),
                description: "List all shortcuts".to_string(),
            },
            HttpRoute {
                method: HttpMethod::Get,
                path: "/{name}".to_string(),
                handler_id: "redirect".to_string(),
                description: "Redirect to shortcut URL".to_string(),
            },
        ]
    }

    async fn handle_request(&self, req: HttpRequest) -> Result<HttpResponse> {
        match req.handler_id.as_str() {
            "health" => Ok(HttpResponse::ok("OK")),
            "list" => {
                let mut entries: Vec<ShortcutEntry> = shortcuts()
                    .iter()
                    .map(|(name, url)| ShortcutEntry {
                        name: name.clone(),
                        url: url.clone(),
                    })
                    .collect();
                entries.sort_by(|a, b| a.name.cmp(&b.name));
                HttpResponse::json(&entries)
            }
            "redirect" => {
                let name = req.path_param("name").unwrap_or_default();
                match shortcuts().get(name) {
                    Some(target) => Ok(HttpResponse::custom(
                        StatusCode::FOUND,
                        [("location".to_string(), target.clone())].into(),
                        "",
                    )),
                    None => {
                        let body = ErrorBody {
                            error: format!("shortcut '{name}' not found"),
                        };
                        let mut resp = HttpResponse::json(&body)?;
                        resp.status = StatusCode::NOT_FOUND;
                        Ok(resp)
                    }
                }
            }
            _ => Ok(HttpResponse::error(StatusCode::NOT_FOUND, "unknown handler")),
        }
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(ShortcutsPlugin)
}

#[no_mangle]
pub fn plugin_create_http() -> Box<dyn HttpRoutes> {
    Box::new(ShortcutsPlugin)
}
