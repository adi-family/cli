//! Interactive TTY prompts using dialoguer

use crate::parser::{Input, InputType};
use dialoguer::{Confirm, Input as DialoguerInput, MultiSelect, Password, Select};
use std::collections::HashMap;
use std::io::{self, IsTerminal};

/// Collect all input values from user via interactive prompts
pub fn collect_inputs(inputs: &[Input]) -> Result<HashMap<String, serde_json::Value>, String> {
    let mut values = HashMap::new();

    // Check if we have a TTY
    if !io::stdin().is_terminal() {
        return Err("Interactive prompts require a TTY".to_string());
    }

    for input in inputs {
        let value = prompt_input(input)?;
        values.insert(input.name.clone(), value);
    }

    Ok(values)
}

/// Prompt for a single input value
fn prompt_input(input: &Input) -> Result<serde_json::Value, String> {
    // Check if value is pre-filled from environment variable
    if let Some(env_var) = &input.env {
        if let Ok(value) = std::env::var(env_var) {
            return Ok(serde_json::Value::String(value));
        }
    }

    match input.input_type {
        InputType::Select => prompt_select(input),
        InputType::Input => prompt_text(input),
        InputType::Confirm => prompt_confirm(input),
        InputType::MultiSelect => prompt_multi_select(input),
        InputType::Password => prompt_password(input),
    }
}

fn prompt_select(input: &Input) -> Result<serde_json::Value, String> {
    let options = input
        .options
        .as_ref()
        .ok_or("Select input requires options")?;

    if options.is_empty() {
        return Err("Select input requires at least one option".to_string());
    }

    let default_index = input
        .default
        .as_ref()
        .and_then(|d| d.as_str())
        .and_then(|default_val| options.iter().position(|o| o == default_val))
        .unwrap_or(0);

    let selection = Select::new()
        .with_prompt(&input.prompt)
        .items(options)
        .default(default_index)
        .interact()
        .map_err(|e| format!("Prompt error: {}", e))?;

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

fn prompt_multi_select(input: &Input) -> Result<serde_json::Value, String> {
    let options = input
        .options
        .as_ref()
        .ok_or("Multi-select input requires options")?;

    if options.is_empty() {
        return Err("Multi-select input requires at least one option".to_string());
    }

    // Determine default selections
    let defaults: Vec<bool> = if let Some(default_val) = &input.default {
        if let Some(arr) = default_val.as_array() {
            let default_strings: Vec<&str> =
                arr.iter().filter_map(|v| v.as_str()).collect();
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
        .items(options)
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
