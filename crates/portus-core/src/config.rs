use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::session::Protocol;

/// Bump this whenever the on-disk shape of [`Config`] changes, and add a
/// migration arm in [`migrate`]. The file is hand-editable, so migrations
/// must be forgiving of missing fields rather than rejecting the file.
pub const CURRENT_SCHEMA_VERSION: u32 = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub schema_version: u32,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub sessions: Vec<SavedSession>,
    #[serde(default)]
    pub settings: Settings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            groups: Vec::new(),
            sessions: Vec::new(),
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
    /// Where this folder sits among its siblings (same `parent_id`), for
    /// drag-and-drop reordering in the sidebar. Fractional on purpose: a
    /// drop between two siblings gets the midpoint of their two values,
    /// which never requires renumbering every other sibling the way a
    /// plain integer index would.
    #[serde(default)]
    pub sort_order: f64,
}

/// A saved, reusable session profile — an SSH/RDP/serial target or a local
/// shell preset that the user has given a name and (for network protocols)
/// stored credentials for, distinct from the ephemeral, unnamed
/// [`crate::session::Session`] a protocol crate spins up when a tab actually
/// connects.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSession {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub group_id: Option<Uuid>,
    pub protocol: Protocol,
    /// Hostname/IP for network protocols, device path for serial. Unused
    /// (empty string) for a saved local-shell preset — see `shell_command`.
    pub address: String,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub baud_rate: Option<u32>,
    #[serde(default)]
    pub auth: AuthMethod,
    /// Local-shell-only: overrides `$SHELL`/`$COMSPEC` when set. Ignored by
    /// every other protocol.
    #[serde(default)]
    pub shell_command: Option<String>,
    /// Local-shell-only: overrides `$HOME` as the starting directory when
    /// set. Ignored by every other protocol.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// Where this session sits among its siblings (same `group_id`), for
    /// drag-and-drop reordering in the sidebar — see [`Group::sort_order`].
    #[serde(default)]
    pub sort_order: f64,
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
    /// when the saved session is deleted.
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
    #[serde(default)]
    pub terminal_colors: TerminalColors,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            terminal_font_family: default_font_family(),
            terminal_font_size: default_font_size(),
            terminal_colors: TerminalColors::default(),
        }
    }
}

/// Per-machine overrides for the terminal's 16-color ANSI palette — never
/// synced anywhere, just read out of this machine's own config.json. Every
/// field is `None` by default, meaning "use xterm.js's own default for that
/// color" (the frontend only sets the corresponding CSS custom property
/// when a field here is actually `Some`, so an untouched config changes
/// nothing about how the terminal looks). Hex strings (e.g. "#8ae234"),
/// validated frontend-side by `<input type="color">`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalColors {
    #[serde(default)]
    pub black: Option<String>,
    #[serde(default)]
    pub red: Option<String>,
    #[serde(default)]
    pub green: Option<String>,
    #[serde(default)]
    pub yellow: Option<String>,
    #[serde(default)]
    pub blue: Option<String>,
    #[serde(default)]
    pub magenta: Option<String>,
    #[serde(default)]
    pub cyan: Option<String>,
    #[serde(default)]
    pub white: Option<String>,
    #[serde(default)]
    pub bright_black: Option<String>,
    #[serde(default)]
    pub bright_red: Option<String>,
    #[serde(default)]
    pub bright_green: Option<String>,
    #[serde(default)]
    pub bright_yellow: Option<String>,
    #[serde(default)]
    pub bright_blue: Option<String>,
    #[serde(default)]
    pub bright_magenta: Option<String>,
    #[serde(default)]
    pub bright_cyan: Option<String>,
    #[serde(default)]
    pub bright_white: Option<String>,
}

/// The CSS generic `monospace` keyword, not a specific font name — it
/// always resolves to whatever real monospace font the OS actually has
/// installed, so it can never hit the "requested font isn't installed,
/// browser silently substitutes a proportional one" failure mode a named
/// font (e.g. "JetBrains Mono") can.
fn default_font_family() -> String {
    "monospace".to_string()
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

    // v2 -> v3: `Config.hosts` was renamed to `Config.sessions` (the `Host`
    // type itself became `SavedSession`) as part of unifying every
    // connection kind under one "session" concept. Same JSON shape, just a
    // renamed top-level key.
    if version < 3 {
        if let Some(obj) = value.as_object_mut() {
            if let Some(hosts) = obj.remove("hosts") {
                obj.insert("sessions".to_string(), hosts);
            }
        }
    }

    // v3 -> v4: added `sortOrder` to both Group and SavedSession for
    // drag-and-drop reordering. A pre-v4 file has no ordering concept at
    // all, so this just assigns each array's existing on-disk order as its
    // initial sortOrder (0, 1, 2, ...) — index-based rather than scoped per
    // group/parent, but since the frontend always filters by group/parent
    // before sorting, relative order within any given list is preserved
    // either way.
    if version < 4 {
        if let Some(obj) = value.as_object_mut() {
            for key in ["groups", "sessions"] {
                if let Some(serde_json::Value::Array(items)) = obj.get_mut(key) {
                    for (index, item) in items.iter_mut().enumerate() {
                        if let Some(item_obj) = item.as_object_mut() {
                            item_obj.entry("sortOrder").or_insert_with(|| serde_json::Value::from(index as f64));
                        }
                    }
                }
            }
        }
    }

    // Future migrations go here, e.g.:
    // if version < 5 { /* rewrite `value` in place */ }

    if let Some(obj) = value.as_object_mut() {
        // "schemaVersion", matching Config's `rename_all = "camelCase"` —
        // this used to write the snake_case "schema_version" instead, a
        // stray key nothing ever read, leaving the real field perpetually
        // stale on disk. Harmless in practice since every migration step
        // above re-checks the freshly-read `version` and applies
        // idempotently, but worth fixing rather than leaving the clutter.
        obj.insert(
            "schemaVersion".to_string(),
            serde_json::Value::from(CURRENT_SCHEMA_VERSION.max(version as u32)),
        );
    }

    Ok(value)
}
