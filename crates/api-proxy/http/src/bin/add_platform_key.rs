//! CLI tool to add platform provider keys.
//!
//! Usage:
//!   add_platform_key <provider_type> <api_key> [base_url]
//!
//! Example:
//!   add_platform_key openai sk-xxxxx
//!   add_platform_key anthropic sk-ant-xxxxx
//!   add_platform_key openrouter sk-or-xxxxx

use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use std::env;

use api_proxy_core::SecretManager;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: add_platform_key <provider_type> <api_key> [base_url]");
        eprintln!();
        eprintln!("Provider types: openai, anthropic, openrouter");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  add_platform_key openai sk-xxxxx");
        eprintln!("  add_platform_key anthropic sk-ant-xxxxx");
        eprintln!("  add_platform_key openrouter sk-or-xxxxx");
        std::process::exit(1);
    }

    let provider_type = &args[1];
    let api_key = &args[2];
    let base_url = args.get(3).map(|s| s.as_str());

    // Validate provider type
    let valid_providers = ["openai", "anthropic", "openrouter", "custom"];
    if !valid_providers.contains(&provider_type.as_str()) {
        eprintln!(
            "Invalid provider type: {}. Valid types: {:?}",
            provider_type, valid_providers
        );
        std::process::exit(1);
    }

    // Get required env vars
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is required");
    let encryption_key = env::var("ENCRYPTION_KEY").expect("ENCRYPTION_KEY is required");

    // Create secret manager and encrypt the key
    let secrets = SecretManager::from_hex(&encryption_key)?;
    let encrypted_key = secrets.encrypt(api_key)?;

    println!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Adding platform key for provider: {}", provider_type);

    // Upsert the platform key
    sqlx::query(
        r#"
        INSERT INTO platform_provider_keys (provider_type, api_key_encrypted, base_url, is_active)
        VALUES ($1, $2, $3, true)
        ON CONFLICT (provider_type) 
        DO UPDATE SET 
            api_key_encrypted = EXCLUDED.api_key_encrypted,
            base_url = EXCLUDED.base_url,
            updated_at = NOW()
        "#,
    )
    .bind(provider_type)
    .bind(&encrypted_key)
    .bind(base_url)
    .execute(&pool)
    .await?;

    println!(
        "Platform key for '{}' added/updated successfully!",
        provider_type
    );

    // List all platform keys
    println!("\nCurrent platform keys:");
    let keys: Vec<(String, bool, Option<String>)> = sqlx::query_as(
        "SELECT provider_type, is_active, base_url FROM platform_provider_keys ORDER BY provider_type"
    )
    .fetch_all(&pool)
    .await?;

    for (provider, active, url) in keys {
        let status = if active { "active" } else { "inactive" };
        let url_str = url.unwrap_or_else(|| "(default)".to_string());
        println!("  - {} [{}] {}", provider, status, url_str);
    }

    Ok(())
}
