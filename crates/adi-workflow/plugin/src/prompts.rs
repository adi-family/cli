//! Interactive TTY prompts using dialoguer

use crate::options::resolve_options;
use crate::parser::{Input, InputType};
use crate::template;
use dialoguer::{Confirm, FuzzySelect, Input as DialoguerInput, MultiSelect, Password, Select};
use std::collections::HashMap;
use std::io::{self, IsTerminal};

/// Collect all input values from user via interactive prompts
///
/// Inputs are collected incrementally - each input's `if` condition is evaluated
/// against previously collected values, so inputs can depend on earlier ones.
pub fn collect_inputs(inputs: &[Input]) -> Result<HashMap<String, serde_json::Value>, String> {
    let mut values = HashMap::new();

    // Check if we have a TTY
    if !io::stdin().is_terminal() {
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

    // Use fuzzy select if autocomplete is enabled
    let selection = if input.autocomplete.unwrap_or(false) {
        FuzzySelect::new()
            .with_prompt(&input.prompt)
            .items(&options)
            .default(default_index)
            .interact()
            .map_err(|e| format!("Prompt error: {}", e))?
    } else {
        Select::new()
            .with_prompt(&input.prompt)
            .items(&options)
            .default(default_index)
            .interact()
            .map_err(|e| format!("Prompt error: {}", e))?
    };

    Ok(serde_json::Value::String(options[selection].clone()))
}

fn prompt_text(input: &Input) -> Result<serde_json::Value, String> {
    let default_value = input
        .default
        .as_ref()
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    let value = if let Some(validation_pattern) = &input.validation {
        let pattern = regex::Regex::new(validation_pattern)
            .map_err(|e| format!("Invalid validation pattern: {}", e))?;
        let pattern_str = validation_pattern.clone();

        let mut builder = DialoguerInput::<String>::new().with_prompt(&input.prompt);

        if let Some(ref default) = default_value {
            builder = builder.default(default.clone());
        }

        builder
            .validate_with(move |input: &String| -> Result<(), String> {
                if pattern.is_match(input) {
                    Ok(())
                } else {
                    Err(format!("Input must match pattern: {}", pattern_str))
                }
            })
            .interact()
            .map_err(|e| format!("Prompt error: {}", e))?
    } else {
        let mut builder = DialoguerInput::<String>::new().with_prompt(&input.prompt);

        if let Some(ref default) = default_value {
            builder = builder.default(default.clone());
        }

        builder
            .interact()
            .map_err(|e| format!("Prompt error: {}", e))?
    };

    Ok(serde_json::Value::String(value))
}

fn prompt_confirm(input: &Input) -> Result<serde_json::Value, String> {
    let default_value = input
        .default
        .as_ref()
        .and_then(|d| d.as_bool())
        .unwrap_or(false);

    let value = Confirm::new()
        .with_prompt(&input.prompt)
        .default(default_value)
        .interact()
        .map_err(|e| format!("Prompt error: {}", e))?;

    Ok(serde_json::Value::Bool(value))
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
    let defaults: Vec<bool> = if let Some(default_val) = &input.default {
        if let Some(arr) = default_val.as_array() {
            let default_strings: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
            options
                .iter()
                .map(|o| default_strings.contains(&o.as_str()))
                .collect()
        } else {
            vec![false; options.len()]
        }
    } else {
        vec![false; options.len()]
    };

    let selections = MultiSelect::new()
        .with_prompt(&input.prompt)
        .items(&options)
        .defaults(&defaults)
        .interact()
        .map_err(|e| format!("Prompt error: {}", e))?;

    let selected: Vec<serde_json::Value> = selections
        .into_iter()
        .map(|i| serde_json::Value::String(options[i].clone()))
        .collect();

    Ok(serde_json::Value::Array(selected))
}

fn prompt_password(input: &Input) -> Result<serde_json::Value, String> {
    let value = Password::new()
        .with_prompt(&input.prompt)
        .interact()
        .map_err(|e| format!("Prompt error: {}", e))?;

    Ok(serde_json::Value::String(value))
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
