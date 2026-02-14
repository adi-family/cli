/// Declare an enum of environment variables with automatic `as_str()` mapping.
///
/// ```
/// # use lib_env_parse::env_vars;
/// env_vars! {
///     DatabaseUrl => "DATABASE_URL",
///     Port        => "PORT",
/// }
/// assert_eq!(EnvVar::DatabaseUrl.as_str(), "DATABASE_URL");
/// ```
#[macro_export]
macro_rules! env_vars {
    ($($variant:ident => $name:literal),* $(,)?) => {
        enum EnvVar { $($variant),* }
        impl EnvVar {
            const fn as_str(&self) -> &'static str {
                match self { $(Self::$variant => $name),* }
            }
        }
    };
}

/// Parse a string as a truthy boolean value.
///
/// Truthy: `"true"`, `"1"`, `"yes"`, `"on"` (case-insensitive)
/// Everything else (including empty string) is falsy.
pub fn is_truthy(val: &str) -> bool {
    let v = val.trim();
    v.eq_ignore_ascii_case("true") || v == "1" || v.eq_ignore_ascii_case("yes") || v.eq_ignore_ascii_case("on")
}

/// Parse a string as a falsy boolean value.
///
/// Falsy: `"false"`, `"0"`, `"no"`, `"off"` (case-insensitive)
/// Everything else (including empty string) is falsy by default.
pub fn is_falsy(val: &str) -> bool {
    let v = val.trim();
    v.eq_ignore_ascii_case("false") || v == "0" || v.eq_ignore_ascii_case("no") || v.eq_ignore_ascii_case("off")
}

/// Read an env var as a boolean, defaulting to `false` if unset.
///
/// Returns `true` only if the env var is set to a truthy value.
pub fn env_bool(key: &str) -> bool {
    std::env::var(key).map(|v| is_truthy(&v)).unwrap_or(false)
}

/// Read an env var as a boolean, defaulting to `true` if unset.
///
/// Returns `false` only if the env var is set to a falsy value.
pub fn env_bool_default_true(key: &str) -> bool {
    std::env::var(key).map(|v| !is_falsy(&v)).unwrap_or(true)
}

/// Read an env var as `Option<String>`.
pub fn env_opt(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// Read an env var as `String`, falling back to a default.
pub fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthy_values() {
        for val in ["true", "TRUE", "True", "1", "yes", "YES", "on", "ON"] {
            assert!(is_truthy(val), "{val} should be truthy");
        }
    }

    #[test]
    fn falsy_values() {
        for val in ["false", "FALSE", "False", "0", "no", "NO", "off", "OFF"] {
            assert!(is_falsy(val), "{val} should be falsy");
        }
    }

    #[test]
    fn non_truthy() {
        for val in ["", "maybe", "2", "false"] {
            assert!(!is_truthy(val), "{val} should not be truthy");
        }
    }

    #[test]
    fn non_falsy() {
        for val in ["", "maybe", "2", "true"] {
            assert!(!is_falsy(val), "{val} should not be falsy");
        }
    }

    #[test]
    fn trimmed() {
        assert!(is_truthy(" true "));
        assert!(is_falsy(" false "));
    }
}
