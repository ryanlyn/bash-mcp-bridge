use std::time::Duration;

use anyhow::{bail, Result};
use tokio::process::Command;
use tokio::time::timeout;

const UNSUPPORTED_SHELL_TOKENS: &[&str] = &["|", ";", "&&", "||", ">", "<", "&"];

fn find_unsupported_shell_syntax(tokens: &[String]) -> Option<&str> {
    for (index, token) in tokens.iter().enumerate() {
        if UNSUPPORTED_SHELL_TOKENS.contains(&token.as_str()) {
            return Some(token);
        }

        if token.ends_with(';') && index + 1 < tokens.len() {
            return Some(";");
        }
    }

    None
}

#[derive(Debug, PartialEq)]
pub struct ParsedCommand {
    pub binary: String,
    pub args: Vec<String>,
}

pub fn parse_command(command: &str) -> Result<ParsedCommand> {
    let command = command.trim();
    if command.is_empty() {
        bail!("command is empty");
    }

    let tokens = shlex::split(command)
        .ok_or_else(|| anyhow::anyhow!("failed to parse command: mismatched quotes"))?;

    if tokens.is_empty() {
        bail!("command is empty after parsing");
    }

    if let Some(token) = find_unsupported_shell_syntax(&tokens) {
        bail!("shell token '{}' is not supported", token);
    }

    Ok(ParsedCommand {
        binary: tokens[0].clone(),
        args: tokens[1..].to_vec(),
    })
}

#[derive(Debug)]
pub struct ExecuteResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub async fn execute(
    command: &str,
    allowed_bins: &[String],
    timeout_secs: u64,
) -> Result<ExecuteResult> {
    let parsed = parse_command(command)?;

    if !allowed_bins.iter().any(|b| b == &parsed.binary) {
        bail!(
            "binary '{}' is not in the allowed list. Allowed: {}",
            parsed.binary,
            allowed_bins.join(", ")
        );
    }

    let future = Command::new(&parsed.binary).args(&parsed.args).output();

    let output = timeout(Duration::from_secs(timeout_secs), future)
        .await
        .map_err(|_| anyhow::anyhow!("command timed out after {}s", timeout_secs))?
        .map_err(|e| anyhow::anyhow!("failed to execute '{}': {}", parsed.binary, e))?;

    Ok(ExecuteResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}
