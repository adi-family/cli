pub fn extract_user_id(access_token: &str) -> Result<String, String> {
    if access_token.is_empty() {
        return Err("Empty access token".to_string());
    }

    let parts: Vec<&str> = access_token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid token format (expected JWT with 3 parts)".to_string());
    }

    match base64_decode_json(parts[1]) {
        Ok(payload) => match payload.get("sub").and_then(|v| v.as_str()) {
            Some(user_id) if !user_id.is_empty() => Ok(user_id.to_string()),
            _ => Err("Token missing 'sub' (user_id) claim".to_string()),
        },
        Err(e) => Err(format!("Failed to decode token payload: {}", e)),
    }
}

pub fn base64url_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

pub fn base64_decode_json(encoded: &str) -> Result<serde_json::Value, String> {
    let standardized = encoded.replace('-', "+").replace('_', "/");

    let padded = match standardized.len() % 4 {
        2 => format!("{}==", standardized),
        3 => format!("{}=", standardized),
        _ => standardized,
    };

    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &padded)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    serde_json::from_slice(&decoded).map_err(|e| format!("JSON parse error: {}", e))
}
