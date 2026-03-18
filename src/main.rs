use anyhow::Result;
use bash_mcp_bridge::config::ConfigStore;
use bash_mcp_bridge::server::BashBridgeServer;
use clap::Parser;
use rmcp::ServiceExt;
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "bash-mcp-bridge", about = "MCP server for safe host command execution")]
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

    let _watcher = config.spawn_watcher()?;
    tracing::info!("watching config file for changes");

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
                .with_graceful_shutdown(async {
                    tokio::signal::ctrl_c().await.unwrap();
                })
                .await?;
        }
        other => anyhow::bail!("unknown transport: {other} (use 'stdio' or 'http')"),
    }

    Ok(())
}
