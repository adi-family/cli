//! Shell command execution

use crate::parser::Step;
use crate::prelude::get_prelude;
use crate::template::{create_env, render, render_env_vars};
use lib_console_output::{debug, info, warn};
use lib_plugin_prelude::t;
use minijinja::Environment;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub fn execute_steps(
    steps: &[Step],
    variables: &HashMap<String, serde_json::Value>,
) -> Result<(), String> {
    let env = create_env();

    for (i, step) in steps.iter().enumerate() {
        // Check condition if present
        if let Some(condition) = &step.condition {
            if !evaluate_condition(&env, condition, variables)? {
                warn(&t!(
                    "workflow-run-step-skipping",
                    "number" => (i + 1).to_string(),
                    "name" => step.name.as_str()
                ));
                continue;
            }
        }

        info(&t!(
            "workflow-run-step-running",
            "number" => (i + 1).to_string(),
            "name" => step.name.as_str()
        ));

        let command = render(&env, &step.run, variables)?;

        let step_env = if let Some(env_vars) = &step.env {
            Some(render_env_vars(&env, env_vars, variables)?)
        } else {
            None
        };

        execute_command(&command, step_env.as_ref())?;
    }

    Ok(())
}

fn evaluate_condition(
    env: &Environment,
    condition: &str,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<bool, String> {
    let template = format!("{{% if {} %}}true{{% else %}}false{{% endif %}}", condition);
    let result = render(env, &template, variables)?;
    Ok(result == "true")
}

fn execute_command(
    command: &str,
    env_vars: Option<&HashMap<String, String>>,
) -> Result<(), String> {
    let shell = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "bash"
    };

    let shell_arg = if cfg!(target_os = "windows") {
        "/C"
    } else {
        "-c"
    };

    let full_command = format!("{}\n{}", get_prelude(), command);

    let mut cmd = Command::new(shell);
    cmd.arg(shell_arg).arg(&full_command);

    if let Some(env) = env_vars {
        for (key, value) in env {
            cmd.env(key, value);
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        cmd.current_dir(cwd);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| t!("workflow-exec-error-spawn", "error" => e.to_string()))?;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                debug(&format!("  {}", line));
            }
        }
    }

    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                warn(&format!("  {}", line));
            }
        }
    }

    let status = child
        .wait()
        .map_err(|e| t!("workflow-exec-error-wait", "error" => e.to_string()))?;

    if !status.success() {
        return Err(t!(
            "workflow-exec-error-exit-code",
            "code" => status.code().unwrap_or(-1).to_string()
        ));
    }

    Ok(())
}
