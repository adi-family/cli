use lib_env_parse::{env_opt, env_vars};
use tracing_subscriber::EnvFilter;
use website::Mode;

env_vars! {
    Port => "PORT",
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("website=info".parse()?),
        )
        .init();

    let mode = match std::env::args().nth(1).as_deref() {
        Some("dev") => Mode::Dev,
        _ => Mode::Prod,
    };

    let port: u16 = env_opt(EnvVar::Port.as_str())
        .and_then(|p| p.parse().ok())
        .unwrap_or(3080);

    website::run_server(port, mode).await
}
