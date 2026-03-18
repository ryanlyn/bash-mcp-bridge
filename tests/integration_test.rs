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

        let store = bash_mcp_bridge::config::ConfigStore::new(Some(f.path()), vec![]).unwrap();
        assert_eq!(store.snapshot().allowed.bins, vec!["gog"]);

        let new_content = r#"
[allowed]
bins = ["gog", "uv", "cargo"]
"#;
        std::fs::write(f.path(), new_content).unwrap();
        store.reload().unwrap();
        assert_eq!(store.snapshot().allowed.bins, vec!["gog", "uv", "cargo"]);
    }

    #[test]
    fn test_config_reload_preserves_allow_overrides() {
        let toml_content = r#"
[allowed]
bins = ["gog"]
"#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let store =
            bash_mcp_bridge::config::ConfigStore::new(Some(f.path()), vec!["echo".into()]).unwrap();
        assert_eq!(store.snapshot().allowed.bins, vec!["echo"]);

        let new_content = r#"
[allowed]
bins = ["curl"]
"#;
        std::fs::write(f.path(), new_content).unwrap();
        store.reload().unwrap();
        assert_eq!(store.snapshot().allowed.bins, vec!["echo"]);
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
        assert!(result.unwrap_err().to_string().contains("shell token"));
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
    fn test_accept_literal_dollar_parens() {
        let parsed = bash_mcp_bridge::executor::parse_command("echo '$(whoami)'").unwrap();
        assert_eq!(parsed.binary, "echo");
        assert_eq!(parsed.args, vec!["$(whoami)"]);
    }

    #[test]
    fn test_accept_literal_backticks() {
        let parsed = bash_mcp_bridge::executor::parse_command("echo '`whoami`'").unwrap();
        assert_eq!(parsed.binary, "echo");
        assert_eq!(parsed.args, vec!["`whoami`"]);
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
    fn test_accept_quoted_pipe_literal() {
        let parsed = bash_mcp_bridge::executor::parse_command(r#"echo "a|b""#).unwrap();
        assert_eq!(parsed.binary, "echo");
        assert_eq!(parsed.args, vec!["a|b"]);
    }

    #[test]
    fn test_accept_url_with_ampersand() {
        let parsed = bash_mcp_bridge::executor::parse_command(
            r#"echo "https://example.com/?q=rust&lang=en""#,
        )
        .unwrap();
        assert_eq!(parsed.binary, "echo");
        assert_eq!(parsed.args, vec!["https://example.com/?q=rust&lang=en"]);
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
        let result = bash_mcp_bridge::executor::execute("echo hello world", &allowed, 30)
            .await
            .unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout.trim(), "hello world");
        assert!(result.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_execute_rejected_binary() {
        let allowed = vec!["echo".to_string()];
        let result = bash_mcp_bridge::executor::execute("curl http://evil.com", &allowed, 30).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not in the allowed list"));
    }

    #[tokio::test]
    async fn test_execute_nonzero_exit() {
        let allowed = vec!["false".to_string()];
        let result = bash_mcp_bridge::executor::execute("false", &allowed, 30)
            .await
            .unwrap();
        assert_ne!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_execute_quoted_literals() {
        let allowed = vec!["echo".to_string()];
        let result = bash_mcp_bridge::executor::execute(
            r#"echo "https://example.com/?q=rust&lang=en" "a|b""#,
            &allowed,
            30,
        )
        .await
        .unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(
            result.stdout.trim(),
            "https://example.com/?q=rust&lang=en a|b"
        );
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
    use tokio::process::{Child, ChildStdin, ChildStdout, Command};

    struct TestServer {
        _config: NamedTempFile,
        child: Child,
        stdin: ChildStdin,
        reader: tokio::io::Lines<BufReader<ChildStdout>>,
    }

    async fn spawn_stdio_server(config: &str) -> TestServer {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(config.as_bytes()).unwrap();

        let mut child = Command::new(env!("CARGO_BIN_EXE_bash-mcp-bridge"))
            .args(["-c", file.path().to_str().unwrap(), "-t", "stdio"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("failed to start server");

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        TestServer {
            _config: file,
            child,
            stdin,
            reader: BufReader::new(stdout).lines(),
        }
    }

    async fn initialize_server(server: &mut TestServer) -> serde_json::Value {
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
        server
            .stdin
            .write_all(format!("{}\n", init_msg).as_bytes())
            .await
            .unwrap();
        let response = server.reader.next_line().await.unwrap().unwrap();

        let initialized = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        server
            .stdin
            .write_all(format!("{}\n", initialized).as_bytes())
            .await
            .unwrap();

        serde_json::from_str(&response).unwrap()
    }

    async fn call_execute(server: &mut TestServer, command: &str) -> serde_json::Value {
        let call_msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "execute",
                "arguments": {"command": command}
            }
        });
        server
            .stdin
            .write_all(format!("{}\n", call_msg).as_bytes())
            .await
            .unwrap();
        let response = server.reader.next_line().await.unwrap().unwrap();
        serde_json::from_str(&response).unwrap()
    }

    #[tokio::test]
    async fn test_e2e_execute_allowed() {
        let config = r#"
[allowed]
bins = ["echo"]
"#;
        let mut server = spawn_stdio_server(config).await;
        let init_response = initialize_server(&mut server).await;
        assert_eq!(
            init_response["result"]["serverInfo"]["name"],
            "bash-mcp-bridge"
        );

        let response = call_execute(&mut server, "echo hello from bridge").await;
        assert_eq!(response["result"]["isError"], false);
        assert_eq!(
            response["result"]["structuredContent"]["stdout"],
            "hello from bridge\n"
        );
        assert_eq!(response["result"]["structuredContent"]["exit_code"], 0);

        server.child.kill().await.ok();
    }

    #[tokio::test]
    async fn test_e2e_execute_nonzero_exit_is_not_tool_error() {
        let config = r#"
[allowed]
bins = ["false"]
"#;
        let mut server = spawn_stdio_server(config).await;
        initialize_server(&mut server).await;

        let response = call_execute(&mut server, "false").await;
        assert_eq!(response["result"]["isError"], false);
        assert_eq!(response["result"]["structuredContent"]["exit_code"], 1);
        assert_eq!(response["result"]["structuredContent"]["stdout"], "");
        assert_eq!(response["result"]["structuredContent"]["stderr"], "");

        server.child.kill().await.ok();
    }

    #[tokio::test]
    async fn test_e2e_execute_rejected() {
        let config = r#"
[allowed]
bins = ["echo"]
"#;
        let mut server = spawn_stdio_server(config).await;
        initialize_server(&mut server).await;

        let response = call_execute(&mut server, "curl http://evil.com").await;
        assert_eq!(response["result"]["isError"], true);
        assert_eq!(
            response["result"]["structuredContent"]["error"],
            "Error: binary 'curl' is not in the allowed list. Allowed: echo"
        );

        server.child.kill().await.ok();
    }
}
