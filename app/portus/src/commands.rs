use bytes::Bytes;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use portus_core::config::{AuthMethod, Config, Group, SavedSession};
use portus_core::session::Protocol;
use portus_rdp::{RdpConnectOptions, RdpEvent};
use portus_sftp::DirEntry;
use portus_ssh::SshConnectOptions;

use crate::adapter::{AppState, SessionCommand};
use crate::rdp_state::RdpState;
use crate::sftp_state::SftpState;

#[tauri::command]
pub fn session_open(
    protocol: String,
    options: Option<serde_json::Value>,
    // Set when this tab is opening a saved session, not an ad-hoc one — the
    // only thing it currently unlocks is scrollback persistence for saved
    // shell presets (see portus_core::scrollback), since an ad-hoc tab has
    // no stable identity to persist scrollback under anyway.
    saved_session_id: Option<Uuid>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.open(&protocol, options.unwrap_or(serde_json::Value::Null), saved_session_id, app)
}

#[tauri::command]
pub fn session_write(session_id: String, data: Vec<u8>, state: State<'_, AppState>) -> Result<(), String> {
    state.send(&session_id, SessionCommand::Write(Bytes::from(data)))
}

#[tauri::command]
pub fn session_resize(session_id: String, cols: u16, rows: u16, state: State<'_, AppState>) -> Result<(), String> {
    state.send(&session_id, SessionCommand::Resize(cols, rows))
}

#[tauri::command]
pub fn session_close(session_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.send(&session_id, SessionCommand::Shutdown)
}

#[tauri::command]
pub fn get_config() -> Result<Config, String> {
    portus_core::config::load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_config(config: Config) -> Result<(), String> {
    portus_core::config::save(&config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_serial_ports() -> Vec<String> {
    portus_serial::list_ports()
}

/// What the frontend sends for the auth half of a session it wants saved —
/// distinct from [`AuthMethod`] because it carries the raw secret rather
/// than a keychain handle. `save_session` is the only place that resolves
/// one into the other.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AuthInput {
    None,
    /// Reuse whatever the session being edited already has stored — an edit
    /// dialog sends this when its credential field was left blank, so a
    /// hostname/username fix doesn't force retyping the password.
    /// Meaningless (falls back to `None`) when saving a brand-new session.
    Unchanged,
    Password { password: String },
    PrivateKey { path: String, passphrase: Option<String> },
}

/// One past the highest `sort_order` among items whose key (`group_id` or
/// `parent_id`) matches `scope` — i.e. "append after every existing
/// sibling." `0.0` when `scope` has no siblings yet.
fn append_sort_order<T>(items: &[T], scope: Option<Uuid>, key: impl Fn(&T) -> Option<Uuid>, order: impl Fn(&T) -> f64) -> f64 {
    let max_existing = items.iter().filter(|item| key(item) == scope).map(order).fold(f64::NEG_INFINITY, f64::max);
    if max_existing.is_finite() {
        max_existing + 1.0
    } else {
        0.0
    }
}

fn resolve_auth(auth: AuthInput, existing: Option<&AuthMethod>) -> Result<AuthMethod, String> {
    match auth {
        AuthInput::Unchanged => Ok(existing.cloned().unwrap_or(AuthMethod::None)),
        AuthInput::None => Ok(AuthMethod::None),
        AuthInput::Password { password } => {
            let handle = portus_core::keychain::store(&password).map_err(|e| e.to_string())?;
            Ok(AuthMethod::Password { credential_handle: handle })
        }
        AuthInput::PrivateKey { path, passphrase } => {
            let passphrase_handle = match passphrase {
                Some(p) if !p.is_empty() => Some(portus_core::keychain::store(&p).map_err(|e| e.to_string())?),
                _ => None,
            };
            Ok(AuthMethod::PrivateKey { path, passphrase_handle })
        }
    }
}

/// Creates or fully replaces a saved session. `auth` is usually a fresh
/// secret to resolve into a new keychain entry, but can also be `Unchanged`
/// to reuse the edited session's existing credential as-is.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn save_session(
    id: Option<Uuid>,
    name: String,
    group_id: Option<Uuid>,
    protocol: Protocol,
    address: String,
    port: Option<u16>,
    username: Option<String>,
    baud_rate: Option<u32>,
    auth: AuthInput,
    shell_command: Option<String>,
    working_dir: Option<String>,
) -> Result<Config, String> {
    let id = id.unwrap_or_else(Uuid::new_v4);
    let mut config = portus_core::config::load().map_err(|e| e.to_string())?;

    let existing_auth = config.sessions.iter().find(|s| s.id == id).map(|s| s.auth.clone());
    let old_handle = existing_auth.as_ref().and_then(|a| a.credential_handle()).map(str::to_string);
    let resolved_auth = resolve_auth(auth, existing_auth.as_ref())?;

    // Replacing a session's secret orphans its old keychain entry unless we
    // clean it up ourselves — but not when `Unchanged` just reused it.
    if let Some(old) = &old_handle {
        if resolved_auth.credential_handle() != Some(old.as_str()) {
            let _ = portus_core::keychain::delete(old);
        }
    }

    // A plain edit (rename, credential change, etc.) keeps its existing
    // position; only a genuinely new session gets appended after its
    // siblings. Reordering itself happens through `reorder_session`.
    let sort_order = config
        .sessions
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.sort_order)
        .unwrap_or_else(|| append_sort_order(&config.sessions, group_id, |s| s.group_id, |s| s.sort_order));

    let saved_session = SavedSession {
        id,
        name,
        group_id,
        protocol,
        address,
        port,
        username,
        baud_rate,
        auth: resolved_auth,
        shell_command,
        working_dir,
        sort_order,
    };

    if let Some(existing) = config.sessions.iter_mut().find(|s| s.id == id) {
        *existing = saved_session;
    } else {
        config.sessions.push(saved_session);
    }

    portus_core::config::save(&config).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn delete_session(saved_session_id: Uuid) -> Result<Config, String> {
    let mut config = portus_core::config::load().map_err(|e| e.to_string())?;
    if let Some(pos) = config.sessions.iter().position(|s| s.id == saved_session_id) {
        let removed = config.sessions.remove(pos);
        if let Some(handle) = removed.auth.credential_handle() {
            let _ = portus_core::keychain::delete(handle);
        }
        let _ = portus_core::scrollback::clear(saved_session_id);
    }
    portus_core::config::save(&config).map_err(|e| e.to_string())?;
    Ok(config)
}

/// Resolves a saved session's stored secret (password or key passphrase)
/// from the keychain, for the frontend to fold into the connect options it
/// sends to `session_open`. `None` if the session has no stored credential.
#[tauri::command]
pub fn resolve_session_secret(saved_session_id: Uuid) -> Result<Option<String>, String> {
    let config = portus_core::config::load().map_err(|e| e.to_string())?;
    let saved_session = config.sessions.iter().find(|s| s.id == saved_session_id).ok_or("saved session not found")?;
    match saved_session.auth.credential_handle() {
        Some(handle) => portus_core::keychain::retrieve(handle).map(Some).map_err(|e| e.to_string()),
        None => Ok(None),
    }
}

// --- Groups --------------------------------------------------------------
// Folders in the sidebar. A saved session's group_id (or a subgroup's
// parent_id) pointing at a deleted group is never left dangling —
// delete_group always un-parents whatever it contained rather than
// cascading.

/// Creates or renames/reparents a saved folder.
#[tauri::command]
pub fn save_group(id: Option<Uuid>, name: String, parent_id: Option<Uuid>) -> Result<Config, String> {
    let id = id.unwrap_or_else(Uuid::new_v4);
    let mut config = portus_core::config::load().map_err(|e| e.to_string())?;

    if let Some(existing) = config.groups.iter_mut().find(|g| g.id == id) {
        existing.name = name;
        existing.parent_id = parent_id;
    } else {
        let sort_order = append_sort_order(&config.groups, parent_id, |g| g.parent_id, |g| g.sort_order);
        config.groups.push(Group { id, name, parent_id, collapsed: false, sort_order });
    }

    portus_core::config::save(&config).map_err(|e| e.to_string())?;
    Ok(config)
}

/// Moves a saved session to a new folder (or the root, if `group_id` is
/// `None`) and/or a new position within that folder — a drag-and-drop in
/// the sidebar. `sort_order` is computed frontend-side, typically as the
/// midpoint between the two siblings the drop landed between.
#[tauri::command]
pub fn reorder_session(session_id: Uuid, group_id: Option<Uuid>, sort_order: f64) -> Result<Config, String> {
    let mut config = portus_core::config::load().map_err(|e| e.to_string())?;
    if let Some(session) = config.sessions.iter_mut().find(|s| s.id == session_id) {
        session.group_id = group_id;
        session.sort_order = sort_order;
    }
    portus_core::config::save(&config).map_err(|e| e.to_string())?;
    Ok(config)
}

/// Moves a saved folder to a new parent (or the root) and/or a new position
/// among its siblings — see [`reorder_session`].
#[tauri::command]
pub fn reorder_group(group_id: Uuid, parent_id: Option<Uuid>, sort_order: f64) -> Result<Config, String> {
    let mut config = portus_core::config::load().map_err(|e| e.to_string())?;
    if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
        group.parent_id = parent_id;
        group.sort_order = sort_order;
    }
    portus_core::config::save(&config).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn delete_group(group_id: Uuid) -> Result<Config, String> {
    let mut config = portus_core::config::load().map_err(|e| e.to_string())?;
    config.groups.retain(|g| g.id != group_id);
    for saved_session in config.sessions.iter_mut() {
        if saved_session.group_id == Some(group_id) {
            saved_session.group_id = None;
        }
    }
    for group in config.groups.iter_mut() {
        if group.parent_id == Some(group_id) {
            group.parent_id = None;
        }
    }
    portus_core::config::save(&config).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn set_group_collapsed(group_id: Uuid, collapsed: bool) -> Result<Config, String> {
    let mut config = portus_core::config::load().map_err(|e| e.to_string())?;
    if let Some(group) = config.groups.iter_mut().find(|g| g.id == group_id) {
        group.collapsed = collapsed;
    }
    portus_core::config::save(&config).map_err(|e| e.to_string())?;
    Ok(config)
}

// --- SFTP --------------------------------------------------------------
// A file panel, not a terminal Session, so it gets plain async
// request/response commands rather than the session bridge's event stream.

#[tauri::command]
pub async fn sftp_connect(options: SshConnectOptions, state: State<'_, SftpState>) -> Result<String, String> {
    let client = portus_sftp::SftpClient::connect(&options).await.map_err(|e| e.to_string())?;
    Ok(state.insert(client))
}

#[tauri::command]
pub async fn sftp_list(id: String, path: String, state: State<'_, SftpState>) -> Result<Vec<DirEntry>, String> {
    state.get(&id)?.list(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_read_file(id: String, path: String, state: State<'_, SftpState>) -> Result<Vec<u8>, String> {
    state.get(&id)?.read_file(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_write_file(
    id: String,
    path: String,
    data: Vec<u8>,
    state: State<'_, SftpState>,
) -> Result<(), String> {
    state.get(&id)?.write_file(&path, &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_remove_file(id: String, path: String, state: State<'_, SftpState>) -> Result<(), String> {
    state.get(&id)?.remove_file(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_create_dir(id: String, path: String, state: State<'_, SftpState>) -> Result<(), String> {
    state.get(&id)?.create_dir(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sftp_remove_dir(id: String, path: String, state: State<'_, SftpState>) -> Result<(), String> {
    state.get(&id)?.remove_dir(&path).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sftp_disconnect(id: String, state: State<'_, SftpState>) -> Result<(), String> {
    state.remove(&id);
    Ok(())
}

// --- RDP -----------------------------------------------------------------
// View-only for now: connect, stream decoded framebuffer updates, and
// disconnect. No input forwarding yet, so no rdp_write/rdp_resize commands.

#[tauri::command]
pub async fn rdp_connect(options: RdpConnectOptions, app: AppHandle, state: State<'_, RdpState>) -> Result<String, String> {
    let id = state.reserve_id();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<RdpEvent>();

    let forward_app = app.clone();
    let forward_id = id.clone();
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            emit_rdp_event(&forward_app, &forward_id, &event);
        }
    });

    let mut client = portus_rdp::RdpClient::default();
    client.connect(options, tx).await.map_err(|e| e.to_string())?;
    state.insert(id.clone(), client);
    Ok(id)
}

#[tauri::command]
pub fn rdp_disconnect(id: String, state: State<'_, RdpState>) -> Result<(), String> {
    state.remove(&id);
    Ok(())
}

fn emit_rdp_event(app: &AppHandle, id: &str, event: &RdpEvent) {
    let kind = match event {
        RdpEvent::Connected { .. } => "connected",
        RdpEvent::Frame(_) => "frame",
        RdpEvent::Disconnected { .. } => "disconnected",
        RdpEvent::Error { .. } => "error",
    };
    let _ = app.emit(&format!("rdp:{id}:{kind}"), event);
}
