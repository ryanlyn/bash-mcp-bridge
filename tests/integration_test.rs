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
