fn main() -> anyhow::Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8022);
    analytics_plugin::run_ingestion_server(port)
}
