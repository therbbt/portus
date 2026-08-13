use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::session::Protocol;

/// Bump this whenever the on-disk shape of [`Config`] changes, and add a
/// migration arm in [`migrate`]. The file is hand-editable, so migrations
/// must be forgiving of missing fields rather than rejecting the file.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub schema_version: u32,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub hosts: Vec<Host>,
    #[serde(default)]
    pub settings: Settings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            groups: Vec::new(),
            hosts: Vec::new(),
            settings: Settings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    #[serde(default)]
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub group_id: Option<Uuid>,
    pub protocol: Protocol,
    pub address: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub username: Option<String>,
    /// Handle into the OS keychain (see [`crate::keychain`]) — never a
    /// literal secret. `None` means "no stored credential, prompt on connect".
    #[serde(default)]
    pub credential_handle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_font_family")]
    pub terminal_font_family: String,
    #[serde(default = "default_font_size")]
    pub terminal_font_size: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            terminal_font_family: default_font_family(),
            terminal_font_size: default_font_size(),
        }
    }
}

fn default_font_family() -> String {
    "JetBrains Mono".to_string()
}

fn default_font_size() -> u16 {
    14
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not determine config directory")]
    NoConfigDir,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid config json: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn config_path() -> Result<PathBuf, ConfigError> {
    let dirs = ProjectDirs::from("com", "portus", "Portus").ok_or(ConfigError::NoConfigDir)?;
    Ok(dirs.config_dir().join("config.json"))
}

pub fn load() -> Result<Config, ConfigError> {
    let path = config_path()?;
    if !path.exists() {
        let cfg = Config::default();
        save(&cfg)?;
        return Ok(cfg);
    }
    let raw = std::fs::read_to_string(&path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let migrated = migrate(value)?;
    Ok(serde_json::from_value(migrated)?)
}

pub fn save(config: &Config) -> Result<(), ConfigError> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Upgrade an on-disk JSON value from whatever `schema_version` it declares
/// up to [`CURRENT_SCHEMA_VERSION`]. A missing `schema_version` is treated as 1.
fn migrate(mut value: serde_json::Value) -> Result<serde_json::Value, ConfigError> {
    let version = value
        .get("schema_version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1);

    // Future migrations go here, e.g.:
    // if version < 2 { /* rewrite `value` in place */ }

    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "schema_version".to_string(),
            serde_json::Value::from(CURRENT_SCHEMA_VERSION.max(version as u32)),
        );
    }

    Ok(value)
}
