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
    /// A short, user-set tag (e.g. "PROD") shown as a `- [SLUG]` suffix
    /// after a tab's title for any saved session nested under this
    /// folder — see TabStrip.svelte, which walks up to the *nearest*
    /// ancestor folder that has one set. `None`/unset means no suffix.
    #[serde(default)]
    pub slug: Option<String>,
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
    /// Local-shell-only: arguments passed to `shell_command` — e.g.
    /// `["-d", "Ubuntu"]` to launch a specific WSL distro via `wsl.exe`.
    /// Ignored by every other protocol.
    #[serde(default)]
    pub shell_args: Option<Vec<String>>,
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

/// A saved, named terminal color scheme — see the matching `Theme` doc
/// comment in bridge.ts for why this is separate from `terminal_colors`
/// (the one set that's actually live) rather than replacing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub colors: TerminalColors,
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
    #[serde(default)]
    pub themes: Vec<Theme>,
    #[serde(default)]
    pub active_theme_id: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            terminal_font_family: default_font_family(),
            terminal_font_size: default_font_size(),
            terminal_colors: TerminalColors::default(),
            themes: Vec::new(),
            active_theme_id: None,
        }
    }
}

/// Per-machine color overrides — never synced anywhere, just read out of
/// this machine's own config.json. Every field is `None` by default,
/// meaning "use the default for that color" (the frontend only sets the
/// corresponding CSS custom property when a field here is actually
/// `Some`, so an untouched config changes nothing). Hex strings (e.g.
/// "#8ae234"), validated frontend-side by `<input type="color">`. Mostly
/// the 16-slot ANSI terminal palette plus a few dedicated (non-ANSI)
/// highlight colors, but also — despite the name — a full set of the
/// app's own UI chrome colors (window_background through status_error
/// below, Settings' "App Interface" group): they live in the same theme
/// so one saved theme covers both a terminal's text and the app chrome
/// around it, rather than needing two separate systems.
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
    /// Not one of the 16 ANSI slots — this overrides `--highlight-green`,
    /// the dedicated "success" color terminalHighlight.ts's own rules use
    /// (HTTP 2xx, an executable file's permission bits, a BGP session
    /// that's Established, ...), independent of whatever `bright_green`
    /// (which tracks the accent, see tokens.css) happens to be.
    #[serde(default)]
    pub highlight_green: Option<String>,
    /// Overrides `--highlight-get`, an HTTP GET request's own color —
    /// independent of `cyan`, which is IP addresses' alone.
    #[serde(default)]
    pub highlight_get: Option<String>,
    /// Overrides `--highlight-url`, a URL's own color — independent of
    /// `blue`, which is MAC addresses' alone.
    #[serde(default)]
    pub highlight_url: Option<String>,
    /// Overrides `--highlight-ipv6`, an IPv6 address's own color —
    /// independent of `cyan`, which is IPv4 addresses' alone.
    #[serde(default)]
    pub highlight_ipv6: Option<String>,
    /// Not a terminal color at all — overrides `--surface-1`, the sidebar
    /// rail's background (also shared by the top action bar, the same
    /// visual zone).
    #[serde(default)]
    pub sidebar_background: Option<String>,
    /// Overrides `--folder-icon-color`, the sidebar's folder icon —
    /// separate from `bright_blue`, which colors a directory's *name*
    /// inside a terminal, not this icon.
    #[serde(default)]
    pub folder_icon: Option<String>,
    /// Overrides `--status-connected`, the color that marks a tab/pane/
    /// session as SSH specifically — separate from the app's one accent,
    /// so this can be customized without also changing buttons, focus
    /// rings, and the active tab indicator.
    #[serde(default)]
    pub ssh_indicator: Option<String>,
    /// Overrides `--surface-0`, the app's outermost background — behind
    /// the sidebar, tab strip, and every panel.
    #[serde(default)]
    pub window_background: Option<String>,
    /// Overrides `--surface-2` — raised panels: the tab strip itself,
    /// dropdown menus, overlays.
    #[serde(default)]
    pub panel_background: Option<String>,
    /// Overrides `--surface-3` — the hover state for sidebar rows, tabs,
    /// and similar list items.
    #[serde(default)]
    pub hover_background: Option<String>,
    /// Overrides `--surface-4` — the active tab and a pressed button.
    #[serde(default)]
    pub active_background: Option<String>,
    /// Overrides `--fg-primary`, the app's main text color.
    #[serde(default)]
    pub text_primary: Option<String>,
    /// Overrides `--fg-secondary`, muted text — session names, field
    /// labels.
    #[serde(default)]
    pub text_secondary: Option<String>,
    /// Overrides `--fg-tertiary`, the dimmest text — hints, secondary
    /// labels, section titles.
    #[serde(default)]
    pub text_tertiary: Option<String>,
    /// Overrides `--fg-disabled`, text on a disabled control.
    #[serde(default)]
    pub text_disabled: Option<String>,
    /// Overrides `--status-connecting`, the status dot/text while a
    /// session is still connecting.
    #[serde(default)]
    pub status_connecting: Option<String>,
    /// Overrides `--status-disconnected`, the status dot for a closed
    /// session.
    #[serde(default)]
    pub status_disconnected: Option<String>,
    /// Overrides `--status-error`, the status dot/text for a failed
    /// session.
    #[serde(default)]
    pub status_error: Option<String>,
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

/// Serializes every `load` + mutate + `save` round trip a command performs
/// (see the app crate's `commands.rs`: save_session/save_group/etc.) into
/// one critical section — callers must hold it for the *entire* sequence,
/// never just around `load` or just around `save`. Without it, two such
/// round trips racing — two rapid UI actions, a saved-session save landing
/// mid-drag-reorder, or just two `#[test]`s in the same binary running on
/// separate threads, as this project's own integration tests do — silently
/// lose whichever one's `save` finishes first: it read a config already
/// stale by the time it wrote, discarding the other's change with no error
/// or warning to show for it.
pub static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    // Write-then-rename rather than a direct `fs::write`: a plain write is a
    // separate truncate + write syscall, so two saves racing (e.g. two
    // near-simultaneous UI actions, or two app windows open at once) can
    // interleave and leave the shorter write's tail end overwritten by
    // nothing, with stale bytes from the longer write still sitting after
    // it — that malformed leftover then fails every future `load()`,
    // silently breaking all further saves. `rename` on the same filesystem
    // is atomic, but only protects a *single* writer's transition to the
    // new content — a shared tmp path would just move the same race one
    // file over, so each save gets its own uniquely-named tmp file and only
    // the rename is contended (whichever save's rename lands last simply
    // wins outright, in full, rather than interleaving byte-for-byte).
    let tmp_path = path.with_extension(format!("json.{}.tmp", Uuid::new_v4()));
    std::fs::write(&tmp_path, json)?;
    std::fs::rename(&tmp_path, &path)?;
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
