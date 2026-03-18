# bash-mcp-bridge

Rust MCP server. Binary whitelist enforced, no shell execution.

## Architecture

- `src/main.rs` - CLI entrypoint (clap), transport selection (stdio/HTTP)
- `src/server.rs` - MCP server handler with `list_allowed` and `execute` tools
- `src/executor.rs` - Command parsing (shlex), shell token rejection, subprocess execution
- `src/config.rs` - TOML config, hot-reload via file watcher, CLI `--allow` overrides
- `src/lib.rs` - Module re-exports

## Development

```
cargo test        # run all tests
cargo clippy      # lint
cargo fmt         # format
```

Tests are in `tests/integration_test.rs` covering config parsing, reload, command parsing, execution, and E2E MCP protocol over stdio.

## Conventions

- All errors use `anyhow`. No panics except for poisoned locks.
- Commands execute via `tokio::process::Command` (no shell). Shell operator tokens are rejected at the token level after `shlex::split`.
- Config file is optional when `--allow` flags are provided. `--allow` overrides survive config reloads.
- Non-zero exit codes are not tool errors - they return structured `{stdout, stderr, exit_code}`.
