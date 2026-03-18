use anyhow::{Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use serde::Deserialize;
use std::path::{Path, PathBuf};
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
    path: Option<PathBuf>,
    allow_overrides: Option<Vec<String>>,
    config: Arc<RwLock<Config>>,
}

impl ConfigStore {
    pub fn new(path: Option<&Path>, allow_overrides: Vec<String>) -> Result<Self> {
        let resolved_path = if let Some(p) = path {
            Some(
                p.canonicalize()
                    .with_context(|| format!("failed to resolve config path: {}", p.display()))?,
            )
        } else {
            None
        };

        let allow_overrides = (!allow_overrides.is_empty()).then_some(allow_overrides);
        let mut config = Self::load_config(resolved_path.as_deref())?;
        Self::apply_allow_overrides(&mut config, allow_overrides.as_deref());

        Ok(Self {
            path: resolved_path,
            allow_overrides,
            config: Arc::new(RwLock::new(config)),
        })
    }

    fn load_config(path: Option<&Path>) -> Result<Config> {
        match path {
            Some(path) => Config::from_file(path),
            None => Ok(Config {
                server: ServerConfig::default(),
                allowed: AllowedConfig { bins: vec![] },
            }),
        }
    }

    fn apply_allow_overrides(config: &mut Config, allow_overrides: Option<&[String]>) {
        if let Some(allow_overrides) = allow_overrides {
            config.allowed.bins = allow_overrides.to_vec();
        }
    }

    pub fn reload(&self) -> Result<()> {
        let path = self
            .path
            .as_deref()
            .context("config reload requires a config path")?;
        let mut new_config = Config::from_file(path)?;
        Self::apply_allow_overrides(&mut new_config, self.allow_overrides.as_deref());
        let mut config = self
            .config
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;
        *config = new_config;
        Ok(())
    }

    pub fn snapshot(&self) -> Config {
        self.config.read().expect("config lock poisoned").clone()
    }

    pub fn spawn_watcher(&self) -> Result<notify::RecommendedWatcher> {
        let store = self.clone();
        let path = self
            .path
            .clone()
            .context("config watcher requires a config path")?;
        let mut watcher =
            notify::recommended_watcher(move |res: std::result::Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        match store.reload() {
                            Ok(()) => {
                                tracing::info!(
                                    bins = ?store.snapshot().allowed.bins,
                                    "config reloaded"
                                )
                            }
                            Err(e) => {
                                tracing::warn!(%e, "failed to reload config, keeping previous")
                            }
                        }
                    }
                }
            })?;
        watcher.watch(path.parent().unwrap_or(&path), RecursiveMode::NonRecursive)?;
        Ok(watcher)
    }
}
