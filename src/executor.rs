use anyhow::{bail, Result};

const SHELL_METACHARACTERS: &[&str] = &["|", ";", "&&", "||", "$(", "`", ">", "<", "&"];

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

    for meta in SHELL_METACHARACTERS {
        if command.contains(meta) {
            bail!("shell metacharacter '{}' is not allowed", meta);
        }
    }

    let tokens = shlex::split(command)
        .ok_or_else(|| anyhow::anyhow!("failed to parse command: mismatched quotes"))?;

    if tokens.is_empty() {
        bail!("command is empty after parsing");
    }

    Ok(ParsedCommand {
        binary: tokens[0].clone(),
        args: tokens[1..].to_vec(),
    })
}
