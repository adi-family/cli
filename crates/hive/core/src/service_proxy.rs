use crate::hive_config::{HiveConfig, get_rollout_ports};
use crate::observability::LogBuffer;
use axum::{
    body::Body,
    extract::{Host, Request, State, WebSocketUpgrade},
    http::Uri,
    response::Response,
    routing::any,
    Router,
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use tokio_tungstenite::tungstenite;
use tracing::{debug, error, info, warn};

type HyperClient = Client<HttpConnector, Body>;

#[derive(Debug, Clone)]
pub struct Route {
    pub service_name: String,
    pub source: String,
    pub host: Option<String>,
    pub path: String,
    pub port: u16,
    pub strip_prefix: bool,
    pub timeout_ms: u64,
}

type RouteSyncCallback = Arc<dyn Fn(&[Route]) + Send + Sync>;

pub struct ServiceProxyState {
    routes: DashMap<Option<String>, Vec<Route>>,
    client: HyperClient,
    log_buffer: OnceLock<Arc<LogBuffer>>,
    show_error_logs: AtomicBool,
    debug_headers: AtomicBool,
    route_sync: OnceLock<RouteSyncCallback>,
}

impl Default for ServiceProxyState {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceProxyState {
    pub fn new() -> Self {
        let client: HyperClient = Client::builder(TokioExecutor::new())
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(32)
            .build_http();

        Self {
            routes: DashMap::new(),
            client,
            log_buffer: OnceLock::new(),
            show_error_logs: AtomicBool::new(true),
            debug_headers: AtomicBool::new(false),
            route_sync: OnceLock::new(),
        }
    }

    pub fn from_config(config: &HiveConfig) -> Self {
        let state = Self::new();
        if let Some(proxy) = &config.proxy {
            state.show_error_logs.store(proxy.show_error_logs, Ordering::Relaxed);
        }
        state.load_source_config("default", config);
        state
    }

    pub fn set_log_buffer(&self, log_buffer: Arc<LogBuffer>) {
        let _ = self.log_buffer.set(log_buffer);
    }

    pub fn set_route_sync(&self, callback: RouteSyncCallback) {
        let _ = self.route_sync.set(callback);
    }

    pub fn set_show_error_logs(&self, show: bool) {
        self.show_error_logs.store(show, Ordering::Relaxed);
    }

    fn get_error_logs(&self, service_fqn: &str) -> Option<Vec<crate::observability::LogLine>> {
        if !self.show_error_logs.load(Ordering::Relaxed) {
            return None;
        }
        self.log_buffer.get().map(|buf| buf.get(service_fqn, Some(50)))
    }

    pub fn clear_routes(&self) {
        self.routes.clear();
    }

    fn clear_source_routes(&self, source: &str) {
        self.routes.iter_mut().for_each(|mut entry| {
            entry.value_mut().retain(|r| r.source != source);
        });
        self.routes.retain(|_, routes| !routes.is_empty());
    }

    pub fn load_source_config(&self, source: &str, config: &HiveConfig) {
        self.clear_source_routes(source);
        if let Some(proxy) = &config.proxy {
            self.show_error_logs.store(proxy.show_error_logs, Ordering::Relaxed);
            self.debug_headers.store(proxy.debug, Ordering::Relaxed);
        }
        info!("Loading routes from config with {} services", config.services.len());
        for (service_name, service_config) in &config.services {
            if let Some(proxy_config) = &service_config.proxy {
                let ports = service_config.rollout.as_ref()
                    .and_then(|r| get_rollout_ports(r).ok())
                    .unwrap_or_default();

                for endpoint in proxy_config.endpoints() {
                    let port = resolve_port(&endpoint.port, &ports)
                        .unwrap_or_else(|| {
                            ports.values().next().copied().unwrap_or(0)
                        });

                    if port == 0 {
                        warn!(
                            "Service {} has proxy config but no resolvable port",
                            service_name
                        );
                        continue;
                    }

                    debug!(
                        "Adding route: {} -> localhost:{} (host: {:?}, path: {})",
                        service_name, port, endpoint.host, endpoint.path
                    );

                    let route = Route {
                        service_name: service_name.clone(),
                        source: source.to_string(),
                        host: endpoint.host.clone(),
                        path: normalize_path(&endpoint.path),
                        port,
                        strip_prefix: endpoint.strip_prefix,
                        timeout_ms: parse_timeout(&endpoint.timeout),
                    };

                    self.add_route(route);
                }
            }
        }
        let all_routes = self.list_routes();
        info!("Loaded {} routes", all_routes.len());

        if let Some(callback) = self.route_sync.get() {
            callback(&all_routes);
        }
    }

    pub fn add_route(&self, route: Route) {
        let host_key = route.host.clone();
        
        self.routes
            .entry(host_key)
            .or_default()
            .push(route);

        self.routes.iter_mut().for_each(|mut entry| {
            entry.value_mut().sort_by(|a, b| b.path.len().cmp(&a.path.len()));
        });
    }

    pub fn remove_service_routes(&self, service_name: &str) {
        self.routes.iter_mut().for_each(|mut entry| {
            entry.value_mut().retain(|r| r.service_name != service_name);
        });
    }

    pub fn find_route(&self, host: Option<&str>, path: &str) -> Option<Route> {
        if let Some(host) = host {
            if let Some(routes) = self.routes.get(&Some(host.to_string())) {
                for route in routes.iter() {
                    if path.starts_with(&route.path) {
                        return Some(route.clone());
                    }
                }
            }
        }

        if let Some(routes) = self.routes.get(&None) {
            for route in routes.iter() {
                if path.starts_with(&route.path) {
                    return Some(route.clone());
                }
            }
        }

        None
    }

    pub fn update_service_port(&self, service_name: &str, port: u16) {
        self.routes.iter_mut().for_each(|mut entry| {
            for route in entry.value_mut().iter_mut() {
                if route.service_name == service_name {
                    route.port = port;
                }
            }
        });
    }

    pub fn list_routes(&self) -> Vec<Route> {
        let mut all_routes = Vec::new();
        for entry in self.routes.iter() {
            all_routes.extend(entry.value().clone());
        }
        all_routes
    }
}

fn resolve_port(port_ref: &Option<String>, ports: &HashMap<String, u16>) -> Option<u16> {
    match port_ref {
        None => ports.get("http").copied(),
        Some(ref_str) => {
            if ref_str.starts_with("{{runtime.port.") && ref_str.ends_with("}}") {
                let port_name = &ref_str[15..ref_str.len() - 2];
                ports.get(port_name).copied()
            } else {
                ref_str.parse().ok()
            }
        }
    }
}

fn normalize_path(path: &str) -> String {
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };
    
    if path.len() > 1 && path.ends_with('/') {
        path[..path.len() - 1].to_string()
    } else {
        path
    }
}

fn parse_timeout(timeout: &Option<String>) -> u64 {
    timeout.as_ref()
        .and_then(|t| crate::service_manager::parse_duration(t))
        .map(|d| d.as_millis() as u64)
        .unwrap_or(60_000)
}

fn compute_target_path(route: &Route, path: &str, query: Option<&str>) -> String {
    let target_path = if route.strip_prefix {
        let stripped = path.strip_prefix(&route.path).unwrap_or(path);
        if stripped.is_empty() || stripped == "/" {
            "/".to_string()
        } else if stripped.starts_with('/') {
            stripped.to_string()
        } else {
            format!("/{}", stripped)
        }
    } else {
        path.to_string()
    };

    match query {
        Some(q) => format!("{}?{}", target_path, q),
        None => target_path,
    }
}

fn is_websocket_upgrade(req: &Request) -> bool {
    req.headers()
        .get(http::header::UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
}

fn axum_to_tungstenite(msg: axum::extract::ws::Message) -> Option<tungstenite::Message> {
    match msg {
        axum::extract::ws::Message::Text(t) => Some(tungstenite::Message::Text(t.to_string())),
        axum::extract::ws::Message::Binary(b) => Some(tungstenite::Message::Binary(b.to_vec())),
        axum::extract::ws::Message::Ping(p) => Some(tungstenite::Message::Ping(p.to_vec())),
        axum::extract::ws::Message::Pong(p) => Some(tungstenite::Message::Pong(p.to_vec())),
        axum::extract::ws::Message::Close(c) => {
            let close_frame = c.map(|cf| tungstenite::protocol::CloseFrame {
                code: tungstenite::protocol::frame::coding::CloseCode::from(cf.code),
                reason: cf.reason.to_string().into(),
            });
            Some(tungstenite::Message::Close(close_frame))
        }
    }
}

fn tungstenite_to_axum(msg: tungstenite::Message) -> Option<axum::extract::ws::Message> {
    match msg {
        tungstenite::Message::Text(t) => Some(axum::extract::ws::Message::Text(t.into())),
        tungstenite::Message::Binary(b) => Some(axum::extract::ws::Message::Binary(b.into())),
        tungstenite::Message::Ping(p) => Some(axum::extract::ws::Message::Ping(p.into())),
        tungstenite::Message::Pong(p) => Some(axum::extract::ws::Message::Pong(p.into())),
        tungstenite::Message::Close(c) => {
            let close_frame = c.map(|cf| axum::extract::ws::CloseFrame {
                code: cf.code.into(),
                reason: cf.reason.to_string().into(),
            });
            Some(axum::extract::ws::Message::Close(close_frame))
        }
        tungstenite::Message::Frame(_) => None,
    }
}

async fn handle_websocket_proxy(
    ws: WebSocketUpgrade,
    target_url: String,
    service_name: String,
    forward_headers: Vec<(String, String)>,
) -> Response {
    ws.on_upgrade(move |client_socket: axum::extract::ws::WebSocket| async move {
        debug!("WebSocket upgrade accepted, connecting to backend: {}", target_url);

        use tungstenite::client::IntoClientRequest;
        let mut request = match target_url.as_str().into_client_request() {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to build WebSocket request for {}: {}", target_url, e);
                return;
            }
        };

        for (name, value) in &forward_headers {
            if let (Ok(header_name), Ok(header_value)) = (
                tungstenite::http::HeaderName::try_from(name.as_str()),
                tungstenite::http::HeaderValue::try_from(value.as_str()),
            ) {
                request.headers_mut().insert(header_name, header_value);
            }
        }

        let backend_result = tokio_tungstenite::connect_async(request).await;
        let (backend_socket, _) = match backend_result {
            Ok(conn) => conn,
            Err(e) => {
                error!("Failed to connect WebSocket to backend {} ({}): {}", target_url, service_name, e);
                return;
            }
        };

        debug!("WebSocket tunnel established for service {}", service_name);

        let (mut client_tx, mut client_rx) = client_socket.split();
        let (mut backend_tx, mut backend_rx) = backend_socket.split();

        let client_to_backend = async {
            while let Some(msg) = client_rx.next().await {
                match msg {
                    Ok(msg) => {
                        if let Some(backend_msg) = axum_to_tungstenite(msg) {
                            if backend_tx.send(backend_msg).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        let backend_to_client = async {
            while let Some(msg) = backend_rx.next().await {
                match msg {
                    Ok(msg) => {
                        if let Some(client_msg) = tungstenite_to_axum(msg) {
                            if client_tx.send(client_msg).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        };

        tokio::select! {
            _ = client_to_backend => {
                let _ = backend_tx.send(tungstenite::Message::Close(None)).await;
                let _ = client_tx.send(axum::extract::ws::Message::Close(None)).await;
            },
            _ = backend_to_client => {
                let _ = client_tx.send(axum::extract::ws::Message::Close(None)).await;
                let _ = backend_tx.send(tungstenite::Message::Close(None)).await;
            },
        }

        debug!("WebSocket tunnel closed for service {}", service_name);
    })
}

async fn service_proxy_handler(
    State(state): State<Arc<ServiceProxyState>>,
    Host(host): Host,
    ws: Option<WebSocketUpgrade>,
    req: Request,
) -> Response {
    let t_start = Instant::now();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| q.to_string());
    let host_str = host.split(':').next();

    let route = match state.find_route(host_str, &path) {
        Some(r) => r,
        None => {
            debug!("No route found for {}:{}", host, path);
            return crate::error_pages::not_found("No matching route", &path, &host, query.as_deref());
        }
    };
    
    debug!("Route found: {} -> port {}", route.service_name, route.port);

    let full_path = compute_target_path(&route, &path, query.as_deref());

    if is_websocket_upgrade(&req) {
        if let Some(ws) = ws {
            let ws_url = format!("ws://127.0.0.1:{}{}", route.port, full_path);
            debug!(
                "WebSocket proxy {} {} -> {} (service: {})",
                req.method(), path, ws_url, route.service_name
            );

            let mut forward_headers = Vec::new();
            if let Some(cookie) = req.headers().get(http::header::COOKIE) {
                if let Ok(value) = cookie.to_str() {
                    forward_headers.push(("cookie".to_string(), value.to_string()));
                }
            }
            if let Some(auth) = req.headers().get(http::header::AUTHORIZATION) {
                if let Ok(value) = auth.to_str() {
                    forward_headers.push(("authorization".to_string(), value.to_string()));
                }
            }

            return handle_websocket_proxy(ws, ws_url, route.service_name.clone(), forward_headers).await;
        }
    }

    let target_uri: Uri = match format!("http://127.0.0.1:{}{}", route.port, full_path).parse() {
        Ok(uri) => uri,
        Err(e) => {
            error!("Invalid target URI: {}", e);
            return crate::error_pages::bad_request(
                &format!("Invalid target URI: {}", e),
                &path,
                &host,
                query.as_deref(),
            );
        }
    };

    debug!(
        "Proxying {} {} -> {} (service: {})",
        req.method(),
        path,
        target_uri,
        route.service_name
    );

    let (mut parts, body) = req.into_parts();
    parts.uri = target_uri;

    parts.headers.remove(http::header::HOST);
    parts.headers.remove(http::header::CONNECTION);
    parts.headers.remove(http::header::TRANSFER_ENCODING);
    parts.headers.remove(http::header::UPGRADE);

    let proxy_req = Request::from_parts(parts, body);

    let t_upstream_start = Instant::now();
    let response = match state.client.request(proxy_req).await {
        Ok(r) => r,
        Err(e) => {
            error!("Proxy request failed for {}: {}", route.service_name, e);
            let logs = state.get_error_logs(&route.service_name);
            return crate::error_pages::bad_gateway(
                &format!("Service unavailable: {}", e),
                &path,
                &host,
                &route.service_name,
                logs.as_deref(),
                query.as_deref(),
            );
        }
    };
    let upstream_ttfb_ms = t_upstream_start.elapsed().as_millis() as u64;
    let total_ms = t_start.elapsed().as_millis() as u64;
    let overhead_ms = total_ms.saturating_sub(upstream_ttfb_ms);

    let (parts, incoming) = response.into_parts();
    let body = Body::from_stream(incoming.into_data_stream());

    let mut res = Response::new(body);
    *res.status_mut() = parts.status;
    *res.headers_mut() = parts.headers;

    if state.debug_headers.load(Ordering::Relaxed) {
        let headers = res.headers_mut();
        headers.insert("x-hive-upstream-ttfb-ms", http::HeaderValue::from(upstream_ttfb_ms));
        headers.insert("x-hive-overhead-ms", http::HeaderValue::from(overhead_ms));
        headers.insert("x-hive-total-ms", http::HeaderValue::from(total_ms));
        let server_timing = format!(
            "upstream;dur={upstream_ttfb_ms}, overhead;dur={overhead_ms}, total;dur={total_ms}"
        );
        if let Ok(val) = http::HeaderValue::from_str(&server_timing) {
            headers.insert("server-timing", val);
        }
    }

    res
}

pub fn create_service_proxy_router(state: Arc<ServiceProxyState>) -> Router {
    Router::new()
        .fallback(any(service_proxy_handler))
        .with_state(state)
}

pub async fn start_service_proxy_server(
    state: Arc<ServiceProxyState>,
    bind_addrs: &[&str],
    activated_listeners: Vec<std::net::TcpListener>,
) -> anyhow::Result<()> {
    let app = create_service_proxy_router(state.clone());

    if !activated_listeners.is_empty() {
        info!(
            "Using {} socket-activated listener(s) instead of binding",
            activated_listeners.len()
        );
        for std_listener in activated_listeners {
            let app_clone = app.clone();
            let local_addr = std_listener
                .local_addr()
                .map(|a| a.to_string())
                .unwrap_or_else(|_| "unknown".to_string());

            tokio::spawn(async move {
                let listener = match tokio::net::TcpListener::from_std(std_listener) {
                    Ok(l) => l,
                    Err(e) => {
                        error!("Failed to convert activated listener ({}): {}", local_addr, e);
                        return;
                    }
                };

                info!("Proxy server listening on {} (socket-activated)", local_addr);
                if let Err(e) = axum::serve(listener, app_clone).await {
                    error!("Proxy server error on {}: {}", local_addr, e);
                }
            });

            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        return Ok(());
    }

    for addr_str in bind_addrs {
        let addr: std::net::SocketAddr = addr_str.parse()
            .map_err(|e| anyhow::anyhow!("Invalid bind address {}: {}", addr_str, e))?;

        let app_clone = app.clone();

        info!("Service proxy will listen on {}", addr);

        let std_listener = std::net::TcpListener::bind(addr)
            .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;
        std_listener.set_nonblocking(true)?;

        tokio::spawn(async move {
            let listener = match tokio::net::TcpListener::from_std(std_listener) {
                Ok(l) => l,
                Err(e) => {
                    error!("Failed to convert listener for {}: {}", addr, e);
                    return;
                }
            };

            info!("Proxy server listening on {}", addr);
            if let Err(e) = axum::serve(listener, app_clone).await {
                error!("Proxy server error on {}: {}", addr, e);
            }
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/api"), "/api");
        assert_eq!(normalize_path("/api/"), "/api");
        assert_eq!(normalize_path("api"), "/api");
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_route_matching() {
        let state = ServiceProxyState::new();
        
        state.add_route(Route {
            service_name: "auth".to_string(),
            source: "test".to_string(),
            host: Some("adi.test".to_string()),
            path: "/api/auth".to_string(),
            port: 8012,
            strip_prefix: true,
            timeout_ms: 60000,
        });

        state.add_route(Route {
            service_name: "web".to_string(),
            source: "test".to_string(),
            host: Some("adi.test".to_string()),
            path: "/".to_string(),
            port: 8013,
            strip_prefix: false,
            timeout_ms: 60000,
        });

        let route = state.find_route(Some("adi.test"), "/api/auth/login");
        assert!(route.is_some());
        assert_eq!(route.unwrap().service_name, "auth");

        let route = state.find_route(Some("adi.test"), "/dashboard");
        assert!(route.is_some());
        assert_eq!(route.unwrap().service_name, "web");

        let route = state.find_route(Some("other.local"), "/api/auth/login");
        assert!(route.is_none());
    }

    #[test]
    fn test_resolve_port() {
        let mut ports = HashMap::new();
        ports.insert("http".to_string(), 8080);
        ports.insert("grpc".to_string(), 9090);

        assert_eq!(resolve_port(&None, &ports), Some(8080));
        assert_eq!(resolve_port(&Some("{{runtime.port.http}}".to_string()), &ports), Some(8080));
        assert_eq!(resolve_port(&Some("{{runtime.port.grpc}}".to_string()), &ports), Some(9090));
        assert_eq!(resolve_port(&Some("3000".to_string()), &ports), Some(3000));
    }
}
