fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8012".to_string())
        .parse()
        .expect("Invalid PORT");
    auth_http::run_server(port)
}
