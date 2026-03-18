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
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {e}"
            ))])),
        }
    }
}

#[tool_handler]
impl ServerHandler for BashBridgeServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "bash-bridge-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
    }
}
