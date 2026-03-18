use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use serde::Deserialize;
use std::path::Path;
use std::sync::{Arc, RwLock};

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

#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: std::path::PathBuf,
    config: Arc<RwLock<Config>>,
}

impl ConfigStore {
    pub fn new(path: Option<&Path>, allow_overrides: Vec<String>) -> Result<Self> {
        let (resolved_path, mut config) = if let Some(p) = path {
            let canonical = p.canonicalize()
                .with_context(|| format!("failed to resolve config path: {}", p.display()))?;
            let cfg = Config::from_file(&canonical)?;
            (Some(canonical), cfg)
        } else {
            (None, Config {
                server: ServerConfig::default(),
                allowed: AllowedConfig { bins: vec![] },
            })
        };

        if !allow_overrides.is_empty() {
            config.allowed.bins = allow_overrides;
        }

        Ok(Self {
            path: resolved_path.unwrap_or_default(),
            config: Arc::new(RwLock::new(config)),
        })
    }

    pub fn reload(&self) -> Result<()> {
        let new_config = Config::from_file(&self.path)?;
        let mut config = self
            .config
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        *config = new_config;
        Ok(())
    }

    pub fn allowed_bins(&self) -> Vec<String> {
        self.config.read().unwrap().allowed.bins.clone()
    }

    pub fn timeout(&self) -> u64 {
        self.config.read().unwrap().server.timeout
    }

    pub fn host(&self) -> String {
        self.config.read().unwrap().server.host.clone()
    }

    pub fn port(&self) -> u16 {
        self.config.read().unwrap().server.port
    }

    pub fn spawn_watcher(&self) -> Result<notify::RecommendedWatcher> {
        let store = self.clone();
        let path = self.path.clone();
        let mut watcher = notify::recommended_watcher(
            move |res: std::result::Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        match store.reload() {
                            Ok(()) => {
                                tracing::info!(bins = ?store.allowed_bins(), "config reloaded")
                            }
                            Err(e) => {
                                tracing::warn!(%e, "failed to reload config, keeping previous")
                            }
                        }
                    }
                }
            },
        )?;
        watcher.watch(path.parent().unwrap_or(&path), RecursiveMode::NonRecursive)?;
        Ok(watcher)
    }
}
