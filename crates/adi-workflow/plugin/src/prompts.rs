//! Interactive TTY prompts using lib-console-output

use crate::options::resolve_options;
use crate::parser::{Input, InputType};
use crate::template;
use lib_console_output::{
    is_interactive, Confirm, Input as ConsoleInput, MultiSelect, Password, Select, SelectOption,
};
use std::collections::HashMap;

/// Collect all input values from user via interactive prompts
///
/// Inputs are collected incrementally - each input's `if` condition is evaluated
/// against previously collected values, so inputs can depend on earlier ones.
pub fn collect_inputs(inputs: &[Input]) -> Result<HashMap<String, serde_json::Value>, String> {
    let mut values = HashMap::new();

    // Check if we have an interactive terminal
    if !is_interactive() {
        return Err("Interactive prompts require a TTY".to_string());
    }

    let env = template::create_env();

    for input in inputs {
        // Check if this input should be shown based on its condition
        if let Some(condition) = &input.condition {
            if !evaluate_condition(&env, condition, &values)? {
                // Condition is false, skip this input
                continue;
            }
        }

        let value = prompt_input(input, &values)?;
        values.insert(input.name.clone(), value);
    }

    Ok(values)
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

    // Use lib-console-output Select
    // Note: lib-console-output doesn't have fuzzy select, using regular select
    let result = Select::new(&input.prompt)
        .options(select_options)
        .default(default_index)
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
}
