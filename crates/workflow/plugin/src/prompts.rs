//! Interactive TTY prompts using lib-console-output

use crate::options::resolve_options;
use crate::parser::{Input, InputType};
use crate::template;
use lib_console_output::{
    debug, is_interactive, Confirm, Input as ConsoleInput, MultiSelect, Password, Select,
    SelectOption,
};
use std::collections::HashMap;

/// Collect all input values from user via interactive prompts with pre-filled values
///
/// Pre-filled values from CLI arguments take precedence over environment variables
/// and interactive prompts. Missing values are prompted interactively (if TTY available)
/// or cause an error (if non-TTY).
///
/// Inputs are collected incrementally - each input's `if` condition is evaluated
/// against previously collected values, so inputs can depend on earlier ones.
pub fn collect_inputs_with_prefilled(
    inputs: &[Input],
    prefilled: HashMap<String, String>,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let mut values = HashMap::new();
    let env = template::create_env();
    let interactive = is_interactive();

    // Track which inputs need prompting
    let mut missing_inputs = Vec::new();

    for input in inputs {
        // Check if this input should be collected based on its condition
        if let Some(condition) = &input.condition {
            if !evaluate_condition(&env, condition, &values)? {
                // Condition is false, skip this input
                continue;
            }
        }

        // Priority: prefilled (CLI args) > env var > prompt/default
        if let Some(prefilled_value) = prefilled.get(&input.name) {
            // Validate and convert pre-filled value
            let value = convert_prefilled_value(input, prefilled_value, &values)?;
            debug(&format!(
                "Using pre-filled value for '{}': {}",
                input.name, prefilled_value
            ));
            values.insert(input.name.clone(), value);
            continue;
        }

        // Check environment variable
        if let Some(env_var) = &input.env {
            if let Ok(env_value) = std::env::var(env_var) {
                let value = convert_prefilled_value(input, &env_value, &values)?;
                debug(&format!(
                    "Using env var {} for '{}': {}",
                    env_var, input.name, env_value
                ));
                values.insert(input.name.clone(), value);
                continue;
            }
        }

        // Need to prompt - check if we can
        if !interactive {
            // In non-interactive mode, try to use default value
            if let Some(default) = &input.default {
                debug(&format!(
                    "Using default value for '{}': {}",
                    input.name, default
                ));
                values.insert(input.name.clone(), default.clone());
                continue;
            }
            // No default available, mark as missing
            missing_inputs.push(input.name.clone());
            continue;
        }

        // Interactive prompt
        let value = prompt_input(input, &values)?;
        values.insert(input.name.clone(), value);
    }

    // If we have missing inputs in non-interactive mode, return error with details
    if !missing_inputs.is_empty() {
        let missing_list = missing_inputs.join(", ");
        return Err(format!(
            "Missing required inputs in non-interactive mode: {}\n\
             Provide them via CLI: {}",
            missing_list,
            missing_inputs
                .iter()
                .map(|name| format!("-i {}=<value>", name))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }

    Ok(values)
}

/// Convert a pre-filled string value to the appropriate JSON type
fn convert_prefilled_value(
    input: &Input,
    value: &str,
    collected_values: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    match input.input_type {
        InputType::Confirm => {
            // Parse boolean
            let bool_val = match value.to_lowercase().as_str() {
                "true" | "yes" | "y" | "1" => true,
                "false" | "no" | "n" | "0" => false,
                _ => {
                    return Err(format!(
                        "Invalid boolean value for '{}': '{}' (use true/false/yes/no)",
                        input.name, value
                    ))
                }
            };
            Ok(serde_json::Value::Bool(bool_val))
        }
        InputType::Select => {
            // Validate against available options
            let options = resolve_options(input, collected_values)?;

            // Check for exact match first
            if options.contains(&value.to_string()) {
                return Ok(serde_json::Value::String(value.to_string()));
            }

            // Check for prefix match (for options like "install - Build and install...")
            for opt in &options {
                if opt.starts_with(value) || opt.split(" - ").next() == Some(value) {
                    return Ok(serde_json::Value::String(opt.clone()));
                }
            }

            Err(format!(
                "Invalid value for '{}': '{}'\nValid options: {}",
                input.name,
                value,
                options.join(", ")
            ))
        }
        InputType::MultiSelect => {
            // Parse comma-separated values
            let selected: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
            let options = resolve_options(input, collected_values)?;

            let mut result = Vec::new();
            for sel in selected {
                // Check exact match
                if options.contains(&sel.to_string()) {
                    result.push(serde_json::Value::String(sel.to_string()));
                    continue;
                }

                // Check prefix match
                let mut found = false;
                for opt in &options {
                    if opt.starts_with(sel) || opt.split(" - ").next() == Some(sel) {
                        result.push(serde_json::Value::String(opt.clone()));
                        found = true;
                        break;
                    }
                }

                if !found {
                    return Err(format!(
                        "Invalid value for '{}': '{}'\nValid options: {}",
                        input.name,
                        sel,
                        options.join(", ")
                    ));
                }
            }

            Ok(serde_json::Value::Array(result))
        }
        InputType::Input | InputType::Password => {
            // Validate against regex if present
            if let Some(pattern) = &input.validation {
                let regex = regex::Regex::new(pattern)
                    .map_err(|e| format!("Invalid validation pattern: {}", e))?;
                if !regex.is_match(value) {
                    return Err(format!(
                        "Value for '{}' doesn't match pattern: {}",
                        input.name, pattern
                    ));
                }
            }
            Ok(serde_json::Value::String(value.to_string()))
        }
    }
}

/// Evaluate a condition template against current values
///
/// Returns true if the condition evaluates to a truthy value
fn evaluate_condition(
    env: &minijinja::Environment,
    condition: &str,
    values: &HashMap<String, serde_json::Value>,
) -> Result<bool, String> {
    let rendered = template::render(env, condition, values)?;
    let trimmed = rendered.trim().to_lowercase();

    // Check for falsy values
    Ok(!trimmed.is_empty() && trimmed != "false" && trimmed != "0" && trimmed != "none")
}

/// Prompt for a single input value
fn prompt_input(
    input: &Input,
    values: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    // Check if value is pre-filled from environment variable
    if let Some(env_var) = &input.env {
        if let Ok(value) = std::env::var(env_var) {
            return Ok(serde_json::Value::String(value));
        }
    }

    match input.input_type {
        InputType::Select => prompt_select(input, values),
        InputType::Input => prompt_text(input),
        InputType::Confirm => prompt_confirm(input),
        InputType::MultiSelect => prompt_multi_select(input, values),
        InputType::Password => prompt_password(input),
    }
}

fn prompt_select(
    input: &Input,
    values: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    // Resolve options dynamically
    let options = resolve_options(input, values)?;

    if options.is_empty() {
        return Err("Select input requires at least one option".to_string());
    }

    let default_index = input
        .default
        .as_ref()
        .and_then(|d| d.as_str())
        .and_then(|default_val| options.iter().position(|o| o == default_val))
        .unwrap_or(0);

    // Build select options
    let select_options: Vec<SelectOption<String>> = options
        .iter()
        .map(|o| SelectOption::new(o.clone(), o.clone()))
        .collect();

    // Use lib-console-output Select with optional filtering
    let result = Select::new(&input.prompt)
        .options(select_options)
        .default(default_index)
        .filterable(input.autocomplete.unwrap_or(false))
        .max_display(input.autocomplete_count)
        .run();

    match result {
        Some(value) => Ok(serde_json::Value::String(value)),
        None => Err("Selection cancelled".to_string()),
    }
}

fn prompt_text(input: &Input) -> Result<serde_json::Value, String> {
    let default_value = input
        .default
        .as_ref()
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    let mut builder = ConsoleInput::new(&input.prompt);

    if let Some(ref default) = default_value {
        builder = builder.default(default.clone());
    }

    // Add validation if present
    if let Some(validation_pattern) = &input.validation {
        let pattern = regex::Regex::new(validation_pattern)
            .map_err(|e| format!("Invalid validation pattern: {}", e))?;
        let pattern_str = validation_pattern.clone();

        builder = builder.validate(move |input: &str| -> Result<(), String> {
            if pattern.is_match(input) {
                Ok(())
            } else {
                Err(format!("Input must match pattern: {}", pattern_str))
            }
        });
    }

    match builder.run() {
        Some(value) => Ok(serde_json::Value::String(value)),
        None => Err("Input cancelled".to_string()),
    }
}

fn prompt_confirm(input: &Input) -> Result<serde_json::Value, String> {
    let default_value = input
        .default
        .as_ref()
        .and_then(|d| d.as_bool())
        .unwrap_or(false);

    let result = Confirm::new(&input.prompt).default(default_value).run();

    match result {
        Some(value) => Ok(serde_json::Value::Bool(value)),
        None => Err("Confirmation cancelled".to_string()),
    }
}

fn prompt_multi_select(
    input: &Input,
    values: &HashMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    // Resolve options dynamically
    let options = resolve_options(input, values)?;

    if options.is_empty() {
        return Err("Multi-select input requires at least one option".to_string());
    }

    // Determine default selections
    let default_indices: Vec<usize> = if let Some(default_val) = &input.default {
        if let Some(arr) = default_val.as_array() {
            let default_strings: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
            options
                .iter()
                .enumerate()
                .filter(|(_, o)| default_strings.contains(&o.as_str()))
                .map(|(i, _)| i)
                .collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    // Build select options
    let select_options: Vec<SelectOption<String>> = options
        .iter()
        .map(|o| SelectOption::new(o.clone(), o.clone()))
        .collect();

    let result = MultiSelect::new(&input.prompt)
        .options(select_options)
        .defaults(default_indices)
        .run();

    match result {
        Some(selected) => {
            let values: Vec<serde_json::Value> = selected
                .into_iter()
                .map(serde_json::Value::String)
                .collect();
            Ok(serde_json::Value::Array(values))
        }
        None => Err("Multi-select cancelled".to_string()),
    }
}

fn prompt_password(input: &Input) -> Result<serde_json::Value, String> {
    let result = Password::new(&input.prompt).run();

    match result {
        Some(value) => Ok(serde_json::Value::String(value)),
        None => Err("Password input cancelled".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::InputType;

    fn make_input(name: &str, input_type: InputType) -> Input {
        Input {
            name: name.to_string(),
            input_type,
            prompt: format!("Enter {}", name),
            options: None,
            options_cmd: None,
            options_source: None,
            autocomplete: None,
            autocomplete_count: None,
            default: None,
            validation: None,
            env: None,
            condition: None,
        }
    }

    #[test]
    fn test_evaluate_condition_truthy() {
        let env = template::create_env();
        let mut values = HashMap::new();
        values.insert(
            "action".to_string(),
            serde_json::Value::String("deploy - Deploy a service".to_string()),
        );

        // Test starts_with filter
        let result =
            evaluate_condition(&env, "{{ action | starts_with('deploy') }}", &values).unwrap();
        assert!(result, "starts_with('deploy') should be true");

        // Test falsy condition
        let result =
            evaluate_condition(&env, "{{ action | starts_with('logs') }}", &values).unwrap();
        assert!(!result, "starts_with('logs') should be false");
    }

    #[test]
    fn test_evaluate_condition_or() {
        let env = template::create_env();
        let mut values = HashMap::new();
        values.insert(
            "action".to_string(),
            serde_json::Value::String("logs - View logs".to_string()),
        );

        // Test OR condition
        let result = evaluate_condition(
            &env,
            "{{ action | starts_with('logs') or action | starts_with('watch') }}",
            &values,
        )
        .unwrap();
        assert!(result, "OR condition should be true for 'logs'");
    }

    #[test]
    fn test_evaluate_condition_empty_values() {
        let env = template::create_env();
        let values = HashMap::new();

        // Undefined variables should evaluate to falsy
        let result = evaluate_condition(&env, "{{ undefined_var }}", &values).unwrap();
        assert!(!result, "undefined var should be falsy");
    }

    #[test]
    fn test_convert_prefilled_confirm_true() {
        let input = make_input("confirm", InputType::Confirm);
        let values = HashMap::new();

        for val in &["true", "yes", "y", "1", "TRUE", "Yes", "Y"] {
            let result = convert_prefilled_value(&input, val, &values).unwrap();
            assert_eq!(result, serde_json::Value::Bool(true), "Failed for: {}", val);
        }
    }

    #[test]
    fn test_convert_prefilled_confirm_false() {
        let input = make_input("confirm", InputType::Confirm);
        let values = HashMap::new();

        for val in &["false", "no", "n", "0", "FALSE", "No", "N"] {
            let result = convert_prefilled_value(&input, val, &values).unwrap();
            assert_eq!(
                result,
                serde_json::Value::Bool(false),
                "Failed for: {}",
                val
            );
        }
    }

    #[test]
    fn test_convert_prefilled_confirm_invalid() {
        let input = make_input("confirm", InputType::Confirm);
        let values = HashMap::new();

        let result = convert_prefilled_value(&input, "maybe", &values);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid boolean value"));
    }

    #[test]
    fn test_convert_prefilled_select_exact_match() {
        let mut input = make_input("action", InputType::Select);
        input.options = Some(vec![
            "install - Build and install".to_string(),
            "build - Build only".to_string(),
        ]);
        let values = HashMap::new();

        let result =
            convert_prefilled_value(&input, "install - Build and install", &values).unwrap();
        assert_eq!(
            result,
            serde_json::Value::String("install - Build and install".to_string())
        );
    }

    #[test]
    fn test_convert_prefilled_select_prefix_match() {
        let mut input = make_input("action", InputType::Select);
        input.options = Some(vec![
            "install - Build and install".to_string(),
            "build - Build only".to_string(),
        ]);
        let values = HashMap::new();

        // Short prefix should match full option
        let result = convert_prefilled_value(&input, "install", &values).unwrap();
        assert_eq!(
            result,
            serde_json::Value::String("install - Build and install".to_string())
        );
    }

    #[test]
    fn test_convert_prefilled_select_invalid() {
        let mut input = make_input("action", InputType::Select);
        input.options = Some(vec!["a".to_string(), "b".to_string()]);
        let values = HashMap::new();

        let result = convert_prefilled_value(&input, "c", &values);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid value"));
    }

    #[test]
    fn test_convert_prefilled_input_text() {
        let input = make_input("name", InputType::Input);
        let values = HashMap::new();

        let result = convert_prefilled_value(&input, "hello world", &values).unwrap();
        assert_eq!(result, serde_json::Value::String("hello world".to_string()));
    }

    #[test]
    fn test_convert_prefilled_input_with_validation() {
        let mut input = make_input("version", InputType::Input);
        input.validation = Some(r"^\d+\.\d+\.\d+$".to_string());
        let values = HashMap::new();

        // Valid semver
        let result = convert_prefilled_value(&input, "1.2.3", &values).unwrap();
        assert_eq!(result, serde_json::Value::String("1.2.3".to_string()));

        // Invalid
        let result = convert_prefilled_value(&input, "invalid", &values);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("doesn't match pattern"));
    }

    #[test]
    fn test_convert_prefilled_multiselect() {
        let mut input = make_input("tags", InputType::MultiSelect);
        input.options = Some(vec![
            "rust".to_string(),
            "cli".to_string(),
            "plugin".to_string(),
        ]);
        let values = HashMap::new();

        let result = convert_prefilled_value(&input, "rust,cli", &values).unwrap();
        assert_eq!(
            result,
            serde_json::Value::Array(vec![
                serde_json::Value::String("rust".to_string()),
                serde_json::Value::String("cli".to_string()),
            ])
        );
    }
}
