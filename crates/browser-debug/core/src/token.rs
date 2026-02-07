//! Debug token generation and validation utilities

use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Generate a debug token for a cocoon/path
///
/// Token format: base64url({ "c": cocoon_id, "p": path, "n": nonce, "t": timestamp, "s": signature })
pub fn generate_debug_token(cocoon_id: &str, path: &str, hive_secret: &str) -> String {
    let nonce = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().timestamp();

    // Create signature
    let data_to_sign = format!("{}{}{}{}", cocoon_id, path, nonce, timestamp);
    let mut mac =
        HmacSha256::new_from_slice(hive_secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(data_to_sign.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    // Create token JSON
    let token_data = serde_json::json!({
        "c": cocoon_id,
        "p": path,
        "n": nonce,
        "t": timestamp,
        "s": signature
    });

    // Encode as base64url
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_data.to_string().as_bytes())
}

/// Validate a debug token and extract cocoon_id
pub fn validate_debug_token(token: &str, hive_secret: &str) -> crate::Result<String> {
    // Decode base64url
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token)
        .map_err(|e| crate::Error::InvalidToken(format!("Base64 decode error: {}", e)))?;

    let token_data: serde_json::Value = serde_json::from_slice(&decoded)
        .map_err(|e| crate::Error::InvalidToken(format!("JSON parse error: {}", e)))?;

    let cocoon_id = token_data
        .get("c")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::Error::InvalidToken("Missing cocoon_id (c)".to_string()))?;
    let path = token_data
        .get("p")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::Error::InvalidToken("Missing path (p)".to_string()))?;
    let nonce = token_data
        .get("n")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::Error::InvalidToken("Missing nonce (n)".to_string()))?;
    let timestamp = token_data
        .get("t")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| crate::Error::InvalidToken("Missing timestamp (t)".to_string()))?;
    let signature = token_data
        .get("s")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::Error::InvalidToken("Missing signature (s)".to_string()))?;

    // Check timestamp (allow 1 hour window)
    let now = chrono::Utc::now().timestamp();
    let token_age = now - timestamp;
    if token_age > 3600 || token_age < -60 {
        return Err(crate::Error::InvalidToken(format!(
            "Token expired or invalid timestamp (age: {}s)",
            token_age
        )));
    }

    // Verify signature
    let data_to_sign = format!("{}{}{}{}", cocoon_id, path, nonce, timestamp);
    let mut mac =
        HmacSha256::new_from_slice(hive_secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(data_to_sign.as_bytes());
    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if signature != expected_signature {
        return Err(crate::Error::InvalidToken(
            "Invalid token signature".to_string(),
        ));
    }

    Ok(cocoon_id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_token() {
        let cocoon_id = "cocoon-123";
        let path = "/app/page";
        let secret = "test-secret-for-hive";

        let token = generate_debug_token(cocoon_id, path, secret);
        let result = validate_debug_token(&token, secret);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), cocoon_id);
    }

    #[test]
    fn test_invalid_signature() {
        let cocoon_id = "cocoon-123";
        let path = "/app/page";
        let secret = "test-secret-for-hive";

        let token = generate_debug_token(cocoon_id, path, secret);
        let result = validate_debug_token(&token, "wrong-secret");

        assert!(result.is_err());
        assert!(matches!(result, Err(crate::Error::InvalidToken(_))));
    }
}
