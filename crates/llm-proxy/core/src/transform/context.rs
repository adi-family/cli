//! Context types for Rhai scripts.

use rhai::{Dynamic, Map};
use serde::{Deserialize, Serialize};

/// Context passed to request transformation scripts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestContext {
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request path (/v1/chat/completions, etc.)
    pub path: String,
    /// Request headers as key-value pairs
    pub headers: Map,
    /// Parsed JSON body
    pub body: Dynamic,
    /// Model name from request (if present)
    pub model: Option<String>,
}

impl RequestContext {
    /// Create a new request context.
    pub fn new(
        method: &str,
        path: &str,
        headers: &http::HeaderMap,
        body: serde_json::Value,
    ) -> Self {
        let mut header_map = Map::new();
        for (key, value) in headers.iter() {
            if let Ok(v) = value.to_str() {
                header_map.insert(key.to_string().into(), Dynamic::from(v.to_string()));
            }
        }

        let model = body
            .get("model")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        Self {
            method: method.to_string(),
            path: path.to_string(),
            headers: header_map,
            body: json_to_dynamic(&body),
            model,
        }
    }

    /// Convert back to JSON body.
    pub fn to_json_body(&self) -> Result<serde_json::Value, String> {
        dynamic_to_json(&self.body)
    }
}

/// Context passed to response transformation scripts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseContext {
    /// HTTP status code
    pub status_code: i64,
    /// Response headers as key-value pairs
    pub headers: Map,
    /// Parsed JSON body
    pub body: Dynamic,
    /// Model name from response (if present)
    pub model: Option<String>,
    /// Input tokens (if available)
    pub input_tokens: Option<i64>,
    /// Output tokens (if available)
    pub output_tokens: Option<i64>,
}

impl ResponseContext {
    /// Create a new response context.
    pub fn new(
        status_code: u16,
        headers: &http::HeaderMap,
        body: serde_json::Value,
        input_tokens: Option<i32>,
        output_tokens: Option<i32>,
    ) -> Self {
        let mut header_map = Map::new();
        for (key, value) in headers.iter() {
            if let Ok(v) = value.to_str() {
                header_map.insert(key.to_string().into(), Dynamic::from(v.to_string()));
            }
        }

        let model = body
            .get("model")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        Self {
            status_code: status_code as i64,
            headers: header_map,
            body: json_to_dynamic(&body),
            model,
            input_tokens: input_tokens.map(|t| t as i64),
            output_tokens: output_tokens.map(|t| t as i64),
        }
    }

    /// Convert back to JSON body.
    pub fn to_json_body(&self) -> Result<serde_json::Value, String> {
        dynamic_to_json(&self.body)
    }
}

/// Convert serde_json::Value to Rhai Dynamic.
pub fn json_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let vec: Vec<Dynamic> = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(vec)
        }
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.insert(k.clone().into(), json_to_dynamic(v));
            }
            Dynamic::from(map)
        }
    }
}

/// Convert Rhai Dynamic back to serde_json::Value.
pub fn dynamic_to_json(value: &Dynamic) -> Result<serde_json::Value, String> {
    if value.is_unit() {
        return Ok(serde_json::Value::Null);
    }

    if let Some(b) = value.clone().try_cast::<bool>() {
        return Ok(serde_json::Value::Bool(b));
    }

    if let Some(i) = value.clone().try_cast::<i64>() {
        return Ok(serde_json::json!(i));
    }

    if let Some(f) = value.clone().try_cast::<f64>() {
        return Ok(serde_json::json!(f));
    }

    if let Some(s) = value.clone().try_cast::<String>() {
        return Ok(serde_json::Value::String(s));
    }

    if let Some(arr) = value.clone().try_cast::<Vec<Dynamic>>() {
        let json_arr: Result<Vec<_>, _> = arr.iter().map(dynamic_to_json).collect();
        return Ok(serde_json::Value::Array(json_arr?));
    }

    if let Some(map) = value.clone().try_cast::<Map>() {
        let mut obj = serde_json::Map::new();
        for (k, v) in map {
            obj.insert(k.to_string(), dynamic_to_json(&v)?);
        }
        return Ok(serde_json::Value::Object(obj));
    }

    // Try to cast as rhai::Array
    if value.is_array() {
        if let Ok(arr) = value.clone().into_array() {
            let json_arr: Result<Vec<serde_json::Value>, String> =
                arr.iter().map(dynamic_to_json).collect();
            return Ok(serde_json::Value::Array(json_arr?));
        }
    }

    // Try to cast as rhai::Map
    if value.is_map() {
        if let Some(map) = value.clone().try_cast::<rhai::Map>() {
            let mut obj = serde_json::Map::new();
            for (k, v) in map.iter() {
                obj.insert(k.to_string(), dynamic_to_json(v)?);
            }
            return Ok(serde_json::Value::Object(obj));
        }
    }

    Err(format!(
        "Cannot convert Dynamic type to JSON: {:?}",
        value.type_name()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_roundtrip() {
        let json = serde_json::json!({
            "model": "gpt-4",
            "messages": [
                {"role": "user", "content": "Hello"}
            ],
            "temperature": 0.7,
            "max_tokens": 100
        });

        let dynamic = json_to_dynamic(&json);
        let back = dynamic_to_json(&dynamic).unwrap();

        assert_eq!(json, back);
    }
}
