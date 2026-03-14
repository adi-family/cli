use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub const MIN_SECRET_LENGTH: usize = 32;

pub fn validate_secret(secret: &str) -> Result<(), String> {
    if secret.len() < MIN_SECRET_LENGTH {
        return Err(format!(
            "Secret too short: {} characters (minimum: {}). Use: openssl rand -base64 36",
            secret.len(),
            MIN_SECRET_LENGTH
        ));
    }

    if secret.chars().all(|c| c.is_numeric()) {
        return Err("Secret must not be only numbers".to_string());
    }

    if secret.to_lowercase() == secret && secret.chars().all(|c| c.is_alphabetic()) {
        return Err("Secret must not be only lowercase letters".to_string());
    }

    if secret.chars().all(|c| c == secret.chars().next().unwrap()) {
        return Err("Secret must not be repetitive characters".to_string());
    }

    let lower = secret.to_lowercase();
    let weak_patterns = [
        "password", "secret", "admin", "12345", "qwerty", "test", "example",
    ];
    for pattern in &weak_patterns {
        if lower.contains(pattern) {
            return Err(format!(
                "Secret contains weak pattern: '{}'. Use cryptographically random secret",
                pattern
            ));
        }
    }

    let unique_chars: std::collections::HashSet<char> = secret.chars().collect();
    if unique_chars.len() < 10 {
        return Err(format!(
            "Secret has insufficient variety: {} unique characters (minimum: 10)",
            unique_chars.len()
        ));
    }

    Ok(())
}

pub fn derive_device_id(secret: &str, salt: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(salt.as_bytes()).expect("HMAC can take key of any size");
    mac.update(secret.as_bytes());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}
