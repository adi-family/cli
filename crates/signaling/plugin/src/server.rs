use axum::{Router, routing::get};
use lib_env_parse::{env_bool, env_opt, env_vars};
use lib_signaling_protocol::IceServer;
use signaling_core::state::AppState;
use std::net::SocketAddr;
use tracing::info;

use crate::ws;

env_vars! {
    HmacSalt => "HMAC_SALT",
    AuthDomain => "AUTH_DOMAIN",
    AllowManualRegistration => "ALLOW_MANUAL_REGISTRATION",
    WebrtcIceServers => "WEBRTC_ICE_SERVERS",
    WebrtcTurnUsername => "WEBRTC_TURN_USERNAME",
    WebrtcTurnCredential => "WEBRTC_TURN_CREDENTIAL",
}

pub fn run_server(port: u16) -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive("signaling_server=info".parse().unwrap()),
            )
            .init();

        let hmac_salt = env_opt(EnvVar::HmacSalt.as_str())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let auth_domain = env_opt(EnvVar::AuthDomain.as_str());
        let allow_manual = env_bool(EnvVar::AllowManualRegistration.as_str());

        info!("Using HMAC salt for device ID derivation (set HMAC_SALT env to persist across restarts)");
        if let Some(ref domain) = auth_domain {
            info!("Auth domain configured: {}", domain);
        }
        info!("Manual registration: {}", if allow_manual { "enabled" } else { "disabled" });

        let ice_servers = parse_ice_servers();
        if !ice_servers.is_empty() {
            info!("Configured {} ICE server(s) for WebRTC", ice_servers.len());
        }
        let ice_servers_json: Vec<serde_json::Value> = ice_servers
            .iter()
            .filter_map(|s| serde_json::to_value(s).ok())
            .collect();

        let state = AppState::new(hmac_salt, auth_domain, allow_manual, ice_servers_json);

        let app = Router::new()
            .route("/ws", get(ws::ws_handler))
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        info!("Signaling server listening on {}", addr);
        println!("Signaling server listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    })
}

fn parse_ice_servers() -> Vec<IceServer> {
    let urls_str = match env_opt(EnvVar::WebrtcIceServers.as_str()) {
        Some(s) if !s.is_empty() => s,
        _ => return vec![],
    };

    let turn_username = env_opt(EnvVar::WebrtcTurnUsername.as_str());
    let turn_credential = env_opt(EnvVar::WebrtcTurnCredential.as_str());

    let stun_urls: Vec<String> = urls_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| s.starts_with("stun:"))
        .collect();

    let turn_urls: Vec<String> = urls_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| s.starts_with("turn:") || s.starts_with("turns:"))
        .collect();

    let mut servers = Vec::new();

    if !stun_urls.is_empty() {
        servers.push(IceServer {
            urls: stun_urls,
            username: None,
            credential: None,
        });
    }

    if !turn_urls.is_empty() {
        servers.push(IceServer {
            urls: turn_urls,
            username: turn_username,
            credential: turn_credential,
        });
    }

    servers
}
