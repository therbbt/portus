use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::session::Protocol;

/// Bump this whenever the on-disk shape of [`Config`] changes, and add a
/// migration arm in [`migrate`]. The file is hand-editable, so migrations
/// must be forgiving of missing fields rather than rejecting the file.
pub const CURRENT_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    #[serde(default)]
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Host {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub group_id: Option<Uuid>,
    pub protocol: Protocol,
    /// Hostname/IP for network protocols, device path for serial.
    pub address: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub baud_rate: Option<u32>,
    #[serde(default)]
    pub auth: AuthMethod,
}

/// Never holds a literal secret — only handles into the OS keychain (see
/// [`crate::keychain`]). What actually gets prompted for on connect (or
/// resolved from the keychain) depends on which variant this is.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AuthMethod {
    /// No stored credential — prompt on every connect.
    #[default]
    None,
    // `rename_all` on the enum only renames the variant tags above
    // ("password", "privateKey") — it does NOT reach into a struct
    // variant's own fields, so each multi-word field name here needs its
    // own `rename` to end up camelCase on the wire like everything else
    // this frontend touches.
    Password {
        #[serde(rename = "credentialHandle")]
        credential_handle: String,
    },
    PrivateKey {
        path: String,
        #[serde(default, rename = "passphraseHandle")]
        passphrase_handle: Option<String>,
    },
}

impl AuthMethod {
    /// The keychain handle this variant stores, if any — used to resolve
    /// the actual secret on connect, and to clean up the keychain entry
    /// when the host is deleted.
    pub fn credential_handle(&self) -> Option<&str> {
        match self {
            AuthMethod::None => None,
            AuthMethod::Password { credential_handle } => Some(credential_handle),
            AuthMethod::PrivateKey { passphrase_handle, .. } => passphrase_handle.as_deref(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// The per-OS config directory Portus owns. Protocol crates that need their
/// own on-disk state (e.g. `portus-ssh`'s known-hosts store) get a sibling
/// file in here rather than each inventing their own location.
pub fn config_dir() -> Result<PathBuf, ConfigError> {
    let dirs = ProjectDirs::from("com", "portus", "Portus").ok_or(ConfigError::NoConfigDir)?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn config_path() -> Result<PathBuf, ConfigError> {
    Ok(config_dir()?.join("config.json"))
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

    // v1 -> v2: `Host.credential_handle: Option<String>` became `Host.auth:
    // AuthMethod`. No rewrite needed here — `#[serde(default)]` on `auth`
    // means a v1 host missing that field just deserializes to
    // `AuthMethod::None`, and the field being dropped, so any old
    // `credential_handle` on the value is silently ignored by serde.

    // Future migrations go here, e.g.:
    // if version < 3 { /* rewrite `value` in place */ }

    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "schema_version".to_string(),
            serde_json::Value::from(CURRENT_SCHEMA_VERSION.max(version as u32)),
        );
    }

    Ok(value)
}
