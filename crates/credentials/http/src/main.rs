fn main() -> anyhow::Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8033);
    credentials_http::run_server(port)
}
