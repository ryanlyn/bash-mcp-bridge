# bash-bridge-mcp Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust MCP server that lets sandboxed agents execute host CLI commands against a binary name whitelist.

**Architecture:** Single-binary Rust MCP server using `rmcp` crate. One tool (`execute`) accepts a command string, validates it against a TOML-configured whitelist of binary names, and runs it as a subprocess. Supports stdio and streamable HTTP transports. Config file is watched for hot-reload.

**Tech Stack:** Rust, rmcp 1.2, tokio, toml, notify (file watcher), schemars, clap

---

## File Structure

```
bash-bridge-mcp/
  Cargo.toml
  config.toml                    # Example/default config
  DESIGN.md                      # Already exists
  src/
    main.rs                      # CLI arg parsing, transport selection, server startup
    config.rs                    # TOML config parsing, hot-reload watcher
    executor.rs                  # Command parsing, validation, subprocess execution
    server.rs                    # MCP server handler with single `execute` tool
  tests/
    integration_test.rs          # E2E tests against the server
```

---

## Chunk 1: Project scaffold and config parsing

### Task 1: Initialize Rust project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

- [ ] **Step 1: Initialize cargo project**

```bash
cd ~/dev/bash-bridge-mcp
cargo init
```

- [ ] **Step 2: Set up Cargo.toml with dependencies**

```toml
[package]
name = "bash-bridge-mcp"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1", features = ["full"] }
rmcp = { version = "1.2", features = ["server", "transport-io", "transport-streamable-http-server", "macros"] }
schemars = "1.0"
serde = { version = "1", features = ["derive"] }
toml = "0.8"
notify = "7"
clap = { version = "4", features = ["derive"] }
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
shlex = "1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
```

- [ ] **Step 3: Create placeholder main.rs**

```rust
fn main() {
    println!("bash-bridge-mcp");
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git init
git add Cargo.toml src/main.rs DESIGN.md
git commit -m "chore: initialize rust project with dependencies"
```

### Task 2: Config parsing

**Files:**
- Create: `src/config.rs`
- Create: `config.toml`
- Create: `tests/integration_test.rs`

- [ ] **Step 1: Write failing test for config parsing**

Create `tests/integration_test.rs`:

```rust
use std::io::Write;
use tempfile::NamedTempFile;

mod config_tests {
    use super::*;

    #[test]
    fn test_parse_valid_config() {
        let toml_content = r#"
[server]
host = "127.0.0.1"
port = 8741
timeout = 120

[allowed]
bins = ["gog", "uv"]
"#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let config = bash_bridge_mcp::config::Config::from_file(f.path()).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8741);
        assert_eq!(config.server.timeout, 120);
        assert_eq!(config.allowed.bins, vec!["gog", "uv"]);
    }

    #[test]
    fn test_parse_config_defaults() {
        let toml_content = r#"
[allowed]
bins = ["gog"]
"#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let config = bash_bridge_mcp::config::Config::from_file(f.path()).unwrap();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8741);
        assert_eq!(config.server.timeout, 120);
    }

    #[test]
    fn test_parse_config_empty_bins_allowed() {
        let toml_content = r#"
[allowed]
bins = []
"#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let config = bash_bridge_mcp::config::Config::from_file(f.path()).unwrap();
        assert!(config.allowed.bins.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test`
Expected: FAIL - module `config` not found

- [ ] **Step 3: Implement config module**

Create `src/config.rs`:

```rust
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    pub allowed: AllowedConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            timeout: default_timeout(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AllowedConfig {
    pub bins: Vec<String>,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8741
}

fn default_timeout() -> u64 {
    120
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        Ok(config)
    }
}
```

Update `src/main.rs` to expose the module as a library:

Create `src/lib.rs`:

```rust
pub mod config;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: All 3 tests PASS

- [ ] **Step 5: Create example config**

Create `config.toml`:

```toml
[server]
host = "127.0.0.1"
port = 8741
timeout = 120

[allowed]
bins = ["gog", "uv"]
```

- [ ] **Step 6: Commit**

```bash
git add src/config.rs src/lib.rs config.toml tests/integration_test.rs
git commit -m "feat: add TOML config parsing with defaults"
```

### Task 3: Config hot-reload

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Write failing test for hot-reload**

Add to `tests/integration_test.rs`:

```rust
mod reload_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_config_reload_updates_bins() {
        let toml_content = r#"
[allowed]
bins = ["gog"]
"#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let store = bash_bridge_mcp::config::ConfigStore::new(f.path()).unwrap();
        assert_eq!(store.allowed_bins(), vec!["gog"]);

        let new_content = r#"
[allowed]
bins = ["gog", "uv", "cargo"]
"#;
        std::fs::write(f.path(), new_content).unwrap();
        store.reload().unwrap();
        assert_eq!(store.allowed_bins(), vec!["gog", "uv", "cargo"]);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test`
Expected: FAIL - `ConfigStore` not found

- [ ] **Step 3: Implement ConfigStore**

Add to `src/config.rs`:

```rust
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: std::path::PathBuf,
    config: Arc<RwLock<Config>>,
}

impl ConfigStore {
    pub fn new(path: &Path) -> Result<Self> {
        let config = Config::from_file(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            config: Arc::new(RwLock::new(config)),
        })
    }

    pub fn reload(&self) -> Result<()> {
        let new_config = Config::from_file(&self.path)?;
        let mut config = self.config.write().map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        *config = new_config;
        Ok(())
    }

    pub fn allowed_bins(&self) -> Vec<String> {
        self.config.read().unwrap().allowed.bins.clone()
    }

    pub fn timeout(&self) -> u64 {
        self.config.read().unwrap().server.timeout
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/config.rs tests/integration_test.rs
git commit -m "feat: add ConfigStore with reload support"
```

---

## Chunk 2: Command executor

### Task 4: Command parsing and validation

**Files:**
- Create: `src/executor.rs`

- [ ] **Step 1: Write failing tests for command validation**

Add to `tests/integration_test.rs`:

```rust
mod executor_tests {
    #[test]
    fn test_parse_simple_command() {
        let parsed = bash_bridge_mcp::executor::parse_command("gog calendar events foo").unwrap();
        assert_eq!(parsed.binary, "gog");
        assert_eq!(parsed.args, vec!["calendar", "events", "foo"]);
    }

    #[test]
    fn test_parse_command_with_quotes() {
        let parsed = bash_bridge_mcp::executor::parse_command(r#"gog calendar create foo --summary "My Event""#).unwrap();
        assert_eq!(parsed.binary, "gog");
        assert_eq!(parsed.args, vec!["calendar", "create", "foo", "--summary", "My Event"]);
    }

    #[test]
    fn test_reject_pipe() {
        let result = bash_bridge_mcp::executor::parse_command("gog events | grep foo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("metacharacter"));
    }

    #[test]
    fn test_reject_semicolon() {
        let result = bash_bridge_mcp::executor::parse_command("gog events; rm -rf /");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_and_chain() {
        let result = bash_bridge_mcp::executor::parse_command("gog events && curl evil.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_or_chain() {
        let result = bash_bridge_mcp::executor::parse_command("gog events || curl evil.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_subshell() {
        let result = bash_bridge_mcp::executor::parse_command("gog events $(whoami)");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_backticks() {
        let result = bash_bridge_mcp::executor::parse_command("gog events `whoami`");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_redirect() {
        let result = bash_bridge_mcp::executor::parse_command("gog events > /tmp/out");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_background() {
        let result = bash_bridge_mcp::executor::parse_command("gog events &");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_empty_command() {
        let result = bash_bridge_mcp::executor::parse_command("");
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test`
Expected: FAIL - module `executor` not found

- [ ] **Step 3: Implement command parser**

Create `src/executor.rs`:

```rust
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
```

Update `src/lib.rs`:

```rust
pub mod config;
pub mod executor;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/executor.rs src/lib.rs tests/integration_test.rs
git commit -m "feat: add command parser with shell metacharacter rejection"
```

### Task 5: Command execution with whitelist check

**Files:**
- Modify: `src/executor.rs`

- [ ] **Step 1: Write failing tests for execution**

Add to `tests/integration_test.rs`:

```rust
mod execution_tests {
    #[tokio::test]
    async fn test_execute_allowed_command() {
        let allowed = vec!["echo".to_string()];
        let result = bash_bridge_mcp::executor::execute("echo hello world", &allowed, 30).await.unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout.trim(), "hello world");
        assert!(result.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_execute_rejected_binary() {
        let allowed = vec!["echo".to_string()];
        let result = bash_bridge_mcp::executor::execute("curl http://evil.com", &allowed, 30).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not in the allowed list"));
    }

    #[tokio::test]
    async fn test_execute_nonzero_exit() {
        let allowed = vec!["false".to_string()];
        let result = bash_bridge_mcp::executor::execute("false", &allowed, 30).await.unwrap();
        assert_ne!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_execute_timeout() {
        let allowed = vec!["sleep".to_string()];
        let result = bash_bridge_mcp::executor::execute("sleep 60", &allowed, 1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_execute_binary_not_found() {
        let allowed = vec!["nonexistent_binary_xyz".to_string()];
        let result = bash_bridge_mcp::executor::execute("nonexistent_binary_xyz", &allowed, 30).await;
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test`
Expected: FAIL - `execute` function not found

- [ ] **Step 3: Implement execute function**

Add to `src/executor.rs`:

```rust
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug)]
pub struct ExecuteResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub async fn execute(command: &str, allowed_bins: &[String], timeout_secs: u64) -> Result<ExecuteResult> {
    let parsed = parse_command(command)?;

    if !allowed_bins.iter().any(|b| b == &parsed.binary) {
        bail!("binary '{}' is not in the allowed list", parsed.binary);
    }

    let future = Command::new(&parsed.binary)
        .args(&parsed.args)
        .output();

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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/executor.rs tests/integration_test.rs
git commit -m "feat: add command execution with whitelist check and timeout"
```

---

## Chunk 3: MCP server

### Task 6: MCP server handler

**Files:**
- Create: `src/server.rs`

- [ ] **Step 1: Implement MCP server with execute tool**

Create `src/server.rs`:

```rust
use crate::config::ConfigStore;
use crate::executor;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, tool, tool_handler, tool_router,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExecuteParams {
    /// The command to execute (e.g. "gog calendar events primary --json")
    pub command: String,
}

#[derive(Clone)]
pub struct BashBridgeServer {
    config: ConfigStore,
    tool_router: ToolRouter<BashBridgeServer>,
}

#[tool_router]
impl BashBridgeServer {
    pub fn new(config: ConfigStore) -> Self {
        Self {
            config,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Execute a command on the host. Only whitelisted binaries are allowed. Shell metacharacters (pipes, redirects, chaining) are rejected.")]
    async fn execute(
        &self,
        Parameters(ExecuteParams { command }): Parameters<ExecuteParams>,
    ) -> Result<CallToolResult, McpError> {
        let allowed = self.config.allowed_bins();
        let timeout = self.config.timeout();

        match executor::execute(&command, &allowed, timeout).await {
            Ok(result) => {
                let mut text = String::new();
                if !result.stdout.is_empty() {
                    text.push_str(&result.stdout);
                }
                if !result.stderr.is_empty() {
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str("STDERR:\n");
                    text.push_str(&result.stderr);
                }
                text.push_str(&format!("\n\nExit code: {}", result.exit_code));

                if result.exit_code == 0 {
                    Ok(CallToolResult::success(vec![Content::text(text)]))
                } else {
                    Ok(CallToolResult::error(vec![Content::text(text)]))
                }
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(
                format!("Error: {e}"),
            )])),
        }
    }
}

#[tool_handler]
impl ServerHandler for BashBridgeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder().enable_tools().build(),
        )
        .with_server_info(Implementation::new("bash-bridge-mcp", env!("CARGO_PKG_VERSION")))
    }
}
```

Update `src/lib.rs`:

```rust
pub mod config;
pub mod executor;
pub mod server;
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add src/server.rs src/lib.rs
git commit -m "feat: add MCP server handler with execute tool"
```

### Task 7: CLI entrypoint with transport selection

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Implement main with clap args**

Replace `src/main.rs`:

```rust
use anyhow::Result;
use bash_bridge_mcp::config::ConfigStore;
use bash_bridge_mcp::server::BashBridgeServer;
use clap::Parser;
use rmcp::ServiceExt;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "bash-bridge-mcp", about = "MCP server for safe host command execution")]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Transport: "stdio" or "http"
    #[arg(short, long, default_value = "stdio")]
    transport: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let config = ConfigStore::new(&cli.config)?;

    tracing::info!(
        bins = ?config.allowed_bins(),
        "loaded config"
    );

    match cli.transport.as_str() {
        "stdio" => {
            let server = BashBridgeServer::new(config)
                .serve(rmcp::transport::stdio())
                .await?;
            server.waiting().await?;
        }
        "http" => {
            let host = config.host();
            let port = config.port();
            let addr = format!("{host}:{port}");

            use rmcp::transport::streamable_http_server::{
                StreamableHttpService,
                session::local::LocalSessionManager,
            };

            let config_clone = config.clone();
            let service = StreamableHttpService::new(
                move || Ok(BashBridgeServer::new(config_clone.clone())),
                LocalSessionManager::default().into(),
                Default::default(),
            );

            let router = axum::Router::new().nest_service("/mcp", service);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            tracing::info!(%addr, "listening");
            axum::serve(listener, router)
                .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap(); })
                .await?;
        }
        other => anyhow::bail!("unknown transport: {other} (use 'stdio' or 'http')"),
    }

    Ok(())
}
```

Add `host()` and `port()` methods to `ConfigStore` in `src/config.rs`:

```rust
pub fn host(&self) -> String {
    self.config.read().unwrap().server.host.clone()
}

pub fn port(&self) -> u16 {
    self.config.read().unwrap().server.port
}
```

Add `axum` to `Cargo.toml`:

```toml
axum = "0.8"
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 3: Manual smoke test with stdio**

Run: `echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1"}}}' | cargo run -- -c config.toml -t stdio`
Expected: JSON response with server info

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/config.rs Cargo.toml
git commit -m "feat: add CLI entrypoint with stdio and HTTP transport"
```

---

## Chunk 4: Config file watcher

### Task 8: File watcher for hot-reload

**Files:**
- Modify: `src/config.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add file watcher to ConfigStore**

Add to `src/config.rs`:

```rust
use notify::{Watcher, RecursiveMode, Event, EventKind};

impl ConfigStore {
    pub fn spawn_watcher(&self) -> Result<notify::RecommendedWatcher> {
        let store = self.clone();
        let path = self.path.clone();
        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    match store.reload() {
                        Ok(()) => tracing::info!(bins = ?store.allowed_bins(), "config reloaded"),
                        Err(e) => tracing::warn!(%e, "failed to reload config, keeping previous"),
                    }
                }
            }
        })?;
        watcher.watch(path.parent().unwrap_or(&path), RecursiveMode::NonRecursive)?;
        Ok(watcher)
    }
}
```

- [ ] **Step 2: Start watcher in main.rs**

Add after config is loaded in `main()`:

```rust
let _watcher = config.spawn_watcher()?;
tracing::info!("watching config file for changes");
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/config.rs src/main.rs
git commit -m "feat: add config file watcher for hot-reload"
```

---

## Chunk 5: E2E testing

### Task 9: E2E integration tests

**Files:**
- Modify: `tests/integration_test.rs`

- [ ] **Step 1: Write E2E test that starts the server and calls execute via MCP**

Add to `tests/integration_test.rs`:

```rust
mod e2e_tests {
    use std::io::Write;
    use tempfile::NamedTempFile;
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::process::Command;

    #[tokio::test]
    async fn test_e2e_execute_allowed() {
        let config = r#"
[allowed]
bins = ["echo"]
"#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(config.as_bytes()).unwrap();

        let mut child = Command::new(env!("CARGO_BIN_EXE_bash-bridge-mcp"))
            .args(["-c", f.path().to_str().unwrap(), "-t", "stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start server");

        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout).lines();

        // Initialize
        let init_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0.1"}
            }
        });
        stdin.write_all(format!("{}\n", init_msg).as_bytes()).await.unwrap();
        let response = reader.next_line().await.unwrap().unwrap();
        assert!(response.contains("bash-bridge-mcp"));

        // Send initialized notification
        let initialized = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        stdin.write_all(format!("{}\n", initialized).as_bytes()).await.unwrap();

        // Call execute tool
        let call_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "execute",
                "arguments": {"command": "echo hello from bridge"}
            }
        });
        stdin.write_all(format!("{}\n", call_msg).as_bytes()).await.unwrap();
        let response = reader.next_line().await.unwrap().unwrap();
        assert!(response.contains("hello from bridge"));

        child.kill().await.ok();
    }

    #[tokio::test]
    async fn test_e2e_execute_rejected() {
        let config = r#"
[allowed]
bins = ["echo"]
"#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(config.as_bytes()).unwrap();

        let mut child = Command::new(env!("CARGO_BIN_EXE_bash-bridge-mcp"))
            .args(["-c", f.path().to_str().unwrap(), "-t", "stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start server");

        let mut stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout).lines();

        // Initialize
        let init_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0.1"}
            }
        });
        stdin.write_all(format!("{}\n", init_msg).as_bytes()).await.unwrap();
        reader.next_line().await.unwrap();

        let initialized = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        stdin.write_all(format!("{}\n", initialized).as_bytes()).await.unwrap();

        // Call execute with rejected binary
        let call_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "execute",
                "arguments": {"command": "curl http://evil.com"}
            }
        });
        stdin.write_all(format!("{}\n", call_msg).as_bytes()).await.unwrap();
        let response = reader.next_line().await.unwrap().unwrap();
        assert!(response.contains("not in the allowed list"));

        child.kill().await.ok();
    }
}
```

Add `serde_json` to `[dev-dependencies]` in `Cargo.toml`:

```toml
serde_json = "1"
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test`
Expected: All tests PASS (including E2E)

- [ ] **Step 3: Commit**

```bash
git add tests/integration_test.rs Cargo.toml
git commit -m "test: add E2E integration tests for stdio transport"
```

---

## Chunk 6: Polish

### Task 10: Add README and .gitignore

**Files:**
- Create: `.gitignore`
- Create: `README.md`

- [ ] **Step 1: Create .gitignore**

```
/target
```

- [ ] **Step 2: Create README.md**

```markdown
# bash-bridge-mcp

MCP server that lets sandboxed agents execute host CLI commands against a binary name whitelist.

## Usage

```bash
# stdio transport (for Claude Code CLI)
bash-bridge-mcp -c config.toml -t stdio

# HTTP transport (for Cowork)
bash-bridge-mcp -c config.toml -t http
```

## Config

```toml
[server]
host = "127.0.0.1"
port = 8741
timeout = 120

[allowed]
bins = ["gog", "uv"]
```

See [DESIGN.md](DESIGN.md) for full details.
```

- [ ] **Step 3: Commit**

```bash
git add .gitignore README.md
git commit -m "docs: add README and .gitignore"
```

### Task 11: Final verification

- [ ] **Step 1: Clean build**

Run: `cargo build --release`
Expected: Compiles successfully

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: All tests PASS

- [ ] **Step 3: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings
