//! Variable templating engine using minijinja

use minijinja::Environment;
use std::collections::HashMap;

/// Create a template environment with built-in variables and functions
pub fn create_env() -> Environment<'static> {
    let mut env = Environment::new();

    // Add built-in globals
    env.add_global("cwd", std::env::current_dir().unwrap_or_default().to_string_lossy().to_string());
    env.add_global("home", dirs::home_dir().unwrap_or_default().to_string_lossy().to_string());
    env.add_global("date", chrono_date());

    env
}

fn chrono_date() -> String {
    // Simple date format without chrono dependency
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    // Basic date calculation
    let days = secs / 86400;
    let years = (days / 365) + 1970;
    let remaining_days = days % 365;
    let months = remaining_days / 30 + 1;
    let day = remaining_days % 30 + 1;

    format!("{:04}-{:02}-{:02}", years, months, day)
}

/// Render a template string with the given variables
pub fn render(
    env: &Environment,
    template: &str,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<String, String> {
    // Build context from variables with env access
    let mut all_vars = variables.clone();

    // Add environment variable access as nested object
    let env_vars: HashMap<String, String> = std::env::vars().collect();
    all_vars.insert(
        "env".to_string(),
        serde_json::to_value(&env_vars).unwrap_or_default(),
    );

    let ctx = build_context(&all_vars);

    env.render_str(template, ctx)
        .map_err(|e| format!("Template error: {}", e))
}

/// Build minijinja context from variable map
fn build_context(variables: &HashMap<String, serde_json::Value>) -> minijinja::Value {
    let mut map = std::collections::BTreeMap::new();

    for (key, value) in variables {
        map.insert(key.clone(), json_to_minijinja(value));
    }

    minijinja::Value::from_serialize(&map)
}

/// Convert serde_json::Value to minijinja::Value
fn json_to_minijinja(value: &serde_json::Value) -> minijinja::Value {
    match value {
        serde_json::Value::Null => minijinja::Value::UNDEFINED,
        serde_json::Value::Bool(b) => minijinja::Value::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                minijinja::Value::from(i)
            } else if let Some(f) = n.as_f64() {
                minijinja::Value::from(f)
            } else {
                minijinja::Value::UNDEFINED
            }
        }
        serde_json::Value::String(s) => minijinja::Value::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let items: Vec<minijinja::Value> = arr.iter().map(json_to_minijinja).collect();
            minijinja::Value::from(items)
        }
        serde_json::Value::Object(obj) => {
            let map: std::collections::BTreeMap<String, minijinja::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_minijinja(v)))
                .collect();
            minijinja::Value::from_serialize(&map)
        }
    }
}

/// Render environment variables map with templating
pub fn render_env_vars(
    env: &Environment,
    env_vars: &HashMap<String, String>,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, String>, String> {
    let mut result = HashMap::new();

    for (key, value_template) in env_vars {
        let rendered = render(env, value_template, variables)?;
        result.insert(key.clone(), rendered);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_render() {
        let env = create_env();
        let mut vars = HashMap::new();
        vars.insert(
            "name".to_string(),
            serde_json::Value::String("world".to_string()),
        );

        let result = render(&env, "Hello, {{ name }}!", &vars).unwrap();
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_conditional_render() {
        let env = create_env();
        let mut vars = HashMap::new();
        vars.insert("enabled".to_string(), serde_json::Value::Bool(true));

        let result = render(&env, "{% if enabled %}--flag{% endif %}", &vars).unwrap();
        assert_eq!(result, "--flag");
    }
}
