//! Shell command execution

use crate::parser::Step;
use crate::template::{create_env, render, render_env_vars};
use minijinja::Environment;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

/// Execute workflow steps sequentially
pub fn execute_steps(
    steps: &[Step],
    variables: &HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    let env = create_env();

    for (i, step) in steps.iter().enumerate() {
        // Check condition if present
        if let Some(condition) = &step.condition {
            if !evaluate_condition(&env, condition, variables)? {
                println!("  Skipping step {}: {} (condition not met)", i + 1, step.name);
                continue;
            }
        }

        println!("  Running step {}: {}", i + 1, step.name);

        // Render the command template
        let command = render(&env, &step.run, variables)?;

        // Render environment variables if present
        let step_env = if let Some(env_vars) = &step.env {
            Some(render_env_vars(&env, env_vars, variables)?)
        } else {
            None
        };

        // Execute the command
        execute_command(&command, step_env.as_ref())?;
    }

    Ok(())
}

/// Evaluate a condition expression
fn evaluate_condition(
    env: &Environment,
    condition: &str,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<bool, String> {
    // Wrap condition in if statement and render
    let template = format!("{{% if {} %}}true{{% else %}}false{{% endif %}}", condition);
    let result = render(env, &template, variables)?;
    Ok(result == "true")
}

/// Execute a single shell command
fn execute_command(
    command: &str,
    env_vars: Option<&HashMap<String, String>>,
) -> Result<(), String> {
    let shell = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    };

    let shell_arg = if cfg!(target_os = "windows") {
        "/C"
    } else {
        "-c"
    };

    let mut cmd = Command::new(shell);
    cmd.arg(shell_arg).arg(command);

    // Add custom environment variables
    if let Some(env) = env_vars {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    // Inherit current directory
    if let Ok(cwd) = std::env::current_dir() {
        cmd.current_dir(cwd);
    }

    // Stream stdout and stderr
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn command: {}", e))?;

    // Stream stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("    {}", line);
            }
        }
    }

    // Stream stderr
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                eprintln!("    {}", line);
            }
        }
    }

    let status = child.wait().map_err(|e| format!("Failed to wait for command: {}", e))?;

    if !status.success() {
        return Err(format!(
            "Command failed with exit code: {}",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}
