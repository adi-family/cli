//! Generic JSON value abstraction
//!
//! This module provides traits that abstract over different JSON implementations,
//! allowing lib-json-tree to work with any JSON library, not just serde_json.
//!
//! # Example: Using with serde_json (default)
//! ```
//! use lib_json_tree::{JsonValue, flatten_json, JsonTreeState};
//! use serde_json::json;
//!
//! let json = json!({"name": "Alice"});
//! let state = JsonTreeState::new();
//! let nodes = flatten_json(&json, &state);
//! ```

use crate::JsonValueType;

// ==================== JSON Value Trait ====================

/// Trait for abstracting over different JSON implementations
///
/// Implement this trait to use lib-json-tree with a custom JSON library.
///
/// # Required Methods
/// - Type checking: `is_object`, `is_array`, `is_string`, `is_number`, `is_bool`, `is_null`
/// - Accessors: `as_object`, `as_array`, `as_str`, `as_i64`, `as_f64`, `as_bool`
/// - Display: `to_display_string` for primitive value rendering
///
/// # Example Implementation
/// ```ignore
/// impl JsonValue for MyJsonValue {
///     type Object = MyJsonObject;
///     type Array = MyJsonArray;
///
///     fn is_object(&self) -> bool { matches!(self, MyJsonValue::Object(_)) }
///     // ... other methods
/// }
/// ```
pub trait JsonValue: Sized + PartialEq {
    /// The type representing a JSON object (key-value map)
    type ObjectType: JsonObject<Value = Self>;
    /// The type representing a JSON array
    type ArrayType: JsonArray<Value = Self>;

    // Type checking methods
    /// Returns true if this value is a JSON object
    fn is_object(&self) -> bool;
    /// Returns true if this value is a JSON array
    fn is_array(&self) -> bool;
    /// Returns true if this value is a JSON string
    fn is_string(&self) -> bool;
    /// Returns true if this value is a JSON number
    fn is_number(&self) -> bool;
    /// Returns true if this value is a JSON boolean
    fn is_bool(&self) -> bool;
    /// Returns true if this value is JSON null
    fn is_null(&self) -> bool;

    // Accessor methods
    /// Returns the value as an object reference, if it is one
    fn as_object(&self) -> Option<&Self::ObjectType>;
    /// Returns the value as an array reference, if it is one
    fn as_array(&self) -> Option<&Self::ArrayType>;
    /// Returns the value as a string slice, if it is one
    fn as_str(&self) -> Option<&str>;
    /// Returns the value as an i64, if it is a number that fits
    fn as_i64(&self) -> Option<i64>;
    /// Returns the value as an f64, if it is a number
    fn as_f64(&self) -> Option<f64>;
    /// Returns the value as a boolean, if it is one
    fn as_bool(&self) -> Option<bool>;

    // Display methods
    /// Converts a primitive value to its display string representation
    ///
    /// Returns `None` for objects and arrays (containers).
    /// For strings, returns the string with quotes (e.g., `"hello"`).
    /// For numbers/bools/null, returns the appropriate representation.
    fn to_display_string(&self) -> Option<String>;

    /// Get the JSON value type for this value
    fn value_type(&self) -> JsonValueType {
        if self.is_object() {
            JsonValueType::Object
        } else if self.is_array() {
            JsonValueType::Array
        } else if self.is_string() {
            JsonValueType::String
        } else if self.is_number() {
            JsonValueType::Number
        } else if self.is_bool() {
            JsonValueType::Bool
        } else {
            JsonValueType::Null
        }
    }

    /// Check if two values have the same type (for diff comparisons)
    fn same_type(&self, other: &Self) -> bool {
        self.value_type() == other.value_type()
    }
}

// ==================== JSON Object Trait ====================

/// Trait for JSON object types (key-value maps)
pub trait JsonObject {
    /// The value type stored in this object
    type Value: JsonValue;

    /// Returns the number of key-value pairs
    fn len(&self) -> usize;

    /// Returns true if the object has no entries
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a value by key
    fn get(&self, key: &str) -> Option<&Self::Value>;

    /// Iterate over all keys
    fn keys(&self) -> impl Iterator<Item = &str>;

    /// Iterate over all values
    fn values(&self) -> impl Iterator<Item = &Self::Value>;

    /// Iterate over all key-value pairs
    fn iter(&self) -> impl Iterator<Item = (&str, &Self::Value)>;
}

// ==================== JSON Array Trait ====================

/// Trait for JSON array types
pub trait JsonArray {
    /// The value type stored in this array
    type Value: JsonValue;

    /// Returns the number of elements
    fn len(&self) -> usize;

    /// Returns true if the array is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get an element by index
    fn get(&self, index: usize) -> Option<&Self::Value>;

    /// Iterate over all elements
    fn iter(&self) -> impl Iterator<Item = &Self::Value>;
}

// ==================== serde_json Implementation ====================

use serde_json::{Map, Value};

impl JsonValue for Value {
    type ObjectType = Map<String, Value>;
    type ArrayType = Vec<Value>;

    fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    fn as_object(&self) -> Option<&Self::ObjectType> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&Self::ArrayType> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => n.as_f64(),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    fn to_display_string(&self) -> Option<String> {
        match self {
            Value::Object(_) | Value::Array(_) => None,
            Value::String(s) => Some(format!("\"{}\"", s)),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            Value::Null => Some("null".to_string()),
        }
    }
}

impl JsonObject for Map<String, Value> {
    type Value = Value;

    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, key: &str) -> Option<&Self::Value> {
        self.get(key)
    }

    fn keys(&self) -> impl Iterator<Item = &str> {
        self.keys().map(|s| s.as_str())
    }

    fn values(&self) -> impl Iterator<Item = &Self::Value> {
        self.values()
    }

    fn iter(&self) -> impl Iterator<Item = (&str, &Self::Value)> {
        self.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl JsonArray for Vec<Value> {
    type Value = Value;

    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, index: usize) -> Option<&Self::Value> {
        <[Value]>::get(self, index)
    }

    fn iter(&self) -> impl Iterator<Item = &Self::Value> {
        <[Value]>::iter(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_json_value_type_detection() {
        let obj = json!({"key": "value"});
        assert!(obj.is_object());
        assert!(!obj.is_array());

        let arr = json!([1, 2, 3]);
        assert!(arr.is_array());
        assert!(!arr.is_object());

        let s = json!("hello");
        assert!(s.is_string());

        let n = json!(42);
        assert!(n.is_number());

        let b = json!(true);
        assert!(b.is_bool());

        let null = json!(null);
        assert!(null.is_null());
    }

    #[test]
    fn test_json_value_accessors() {
        let obj = json!({"key": "value"});
        assert!(obj.as_object().is_some());
        assert_eq!(obj.as_object().unwrap().len(), 1);

        let arr = json!([1, 2, 3]);
        assert!(arr.as_array().is_some());
        assert_eq!(arr.as_array().unwrap().len(), 3);

        let s = json!("hello");
        assert_eq!(s.as_str(), Some("hello"));

        let n = json!(42);
        assert_eq!(n.as_i64(), Some(42));
        assert_eq!(n.as_f64(), Some(42.0));

        let b = json!(true);
        assert_eq!(b.as_bool(), Some(true));
    }

    #[test]
    fn test_json_object_trait() {
        let obj = json!({"a": 1, "b": 2});
        let map = obj.as_object().unwrap();

        assert_eq!(JsonObject::len(map), 2);
        assert!(!JsonObject::is_empty(map));
        assert!(JsonObject::get(map, "a").is_some());
        assert!(JsonObject::get(map, "c").is_none());

        // Test that keys() returns &str via the trait
        assert!(JsonObject::keys(map).any(|k| k == "a"));
        assert!(JsonObject::keys(map).any(|k| k == "b"));
    }

    #[test]
    fn test_json_array_trait() {
        let arr = json!([1, 2, 3]);
        let vec = arr.as_array().unwrap();

        assert_eq!(vec.len(), 3);
        assert!(!vec.is_empty());
        assert!(vec.get(0).is_some());
        assert!(vec.get(5).is_none());
    }

    #[test]
    fn test_value_type() {
        assert_eq!(json!({"a": 1}).value_type(), JsonValueType::Object);
        assert_eq!(json!([1, 2]).value_type(), JsonValueType::Array);
        assert_eq!(json!("hello").value_type(), JsonValueType::String);
        assert_eq!(json!(42).value_type(), JsonValueType::Number);
        assert_eq!(json!(true).value_type(), JsonValueType::Bool);
        assert_eq!(json!(null).value_type(), JsonValueType::Null);
    }

    #[test]
    fn test_same_type() {
        let a = json!({"a": 1});
        let b = json!({"b": 2});
        let c = json!([1, 2]);

        assert!(a.same_type(&b));
        assert!(!a.same_type(&c));
    }
}
