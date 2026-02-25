fn main() -> anyhow::Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8030);
    balance_http::run_server(port)
}
