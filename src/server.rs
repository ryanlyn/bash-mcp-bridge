use crate::config::ConfigStore;
use crate::executor;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars, serde_json, tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ExecuteParams {
    /// The command to execute (e.g. "gh pr list --json number,title")
    pub command: String,
}

#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct ExecuteOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
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

    fn render_execute_output(result: &ExecuteOutput) -> String {
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
        if !text.is_empty() {
            text.push_str("\n\n");
        }
        text.push_str(&format!("Exit code: {}", result.exit_code));
        text
    }

    fn structured_success(result: ExecuteOutput) -> Result<CallToolResult, McpError> {
        let text = Self::render_execute_output(&result);
        let structured_content = serde_json::to_value(&result).map_err(|e| {
            McpError::internal_error(format!("failed to serialize execute result: {e}"), None)
        })?;
        let mut tool_result = CallToolResult::structured(structured_content);
        tool_result.content = vec![Content::text(text)];
        Ok(tool_result)
    }

    fn structured_error(message: String) -> CallToolResult {
        let text = message.clone();
        let structured_content = serde_json::json!({ "error": message });
        let mut tool_result = CallToolResult::structured_error(structured_content);
        tool_result.content = vec![Content::text(text)];
        tool_result
    }

    #[tool(
        description = "List the binaries that are allowed to be executed, and usage hints for each. Call this first to understand what commands are available."
    )]
    async fn list_allowed(&self) -> Result<CallToolResult, McpError> {
        let allowed = self.config.snapshot().allowed.bins;
        if allowed.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No binaries are currently allowed.",
            )]));
        }

        let mut text = format!("Allowed binaries: {}\n", allowed.join(", "));
        text.push_str("\nRun any allowed binary with --help to see its usage, e.g.:\n");
        for bin in &allowed {
            text.push_str(&format!("  execute(command: \"{bin} --help\")\n"));
        }
        text.push_str(
            "\nCommands run without a shell. Standalone shell operator tokens like |, &&, >, and & are rejected.",
        );

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    #[tool(
        description = "Execute a command on the host. Only whitelisted binaries are allowed. Commands run without a shell, and standalone shell operator tokens for pipes, redirects, chaining, or backgrounding are rejected. Call list_allowed first to see which binaries are available."
    )]
    async fn execute(
        &self,
        Parameters(ExecuteParams { command }): Parameters<ExecuteParams>,
    ) -> Result<CallToolResult, McpError> {
        let config = self.config.snapshot();

        match executor::execute(&command, &config.allowed.bins, config.server.timeout).await {
            Ok(result) => Self::structured_success(ExecuteOutput {
                stdout: result.stdout,
                stderr: result.stderr,
                exit_code: result.exit_code,
            }),
            Err(e) => Ok(Self::structured_error(format!("Error: {e}"))),
        }
    }
}

#[tool_handler]
impl ServerHandler for BashBridgeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_server_info(
            Implementation::new("bash-mcp-bridge", env!("CARGO_PKG_VERSION")),
        )
    }
}
