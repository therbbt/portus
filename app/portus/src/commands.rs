use bytes::Bytes;
use serde::Deserialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

use portus_core::config::{AuthMethod, Config, Host};
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
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state.open(&protocol, options.unwrap_or(serde_json::Value::Null), app)
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

/// What the frontend sends for the auth half of a host it wants saved —
/// distinct from [`AuthMethod`] because it carries the raw secret rather
/// than a keychain handle. `save_host` is the only place that resolves one
/// into the other.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AuthInput {
    None,
    /// Reuse whatever the host being edited already has stored — an edit
    /// dialog sends this when its credential field was left blank, so a
    /// hostname/username fix doesn't force retyping the password.
    /// Meaningless (falls back to `None`) when saving a brand-new host.
    Unchanged,
    Password { password: String },
    PrivateKey { path: String, passphrase: Option<String> },
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

/// Creates or fully replaces a saved host. `auth` is usually a fresh secret
/// to resolve into a new keychain entry, but can also be `Unchanged` to
/// reuse the edited host's existing credential as-is.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn save_host(
    id: Option<Uuid>,
    name: String,
    group_id: Option<Uuid>,
    protocol: Protocol,
    address: String,
    port: Option<u16>,
    username: Option<String>,
    baud_rate: Option<u32>,
    auth: AuthInput,
) -> Result<Config, String> {
    let id = id.unwrap_or_else(Uuid::new_v4);
    let mut config = portus_core::config::load().map_err(|e| e.to_string())?;

    let existing_auth = config.hosts.iter().find(|h| h.id == id).map(|h| h.auth.clone());
    let old_handle = existing_auth.as_ref().and_then(|a| a.credential_handle()).map(str::to_string);
    let resolved_auth = resolve_auth(auth, existing_auth.as_ref())?;

    // Replacing a host's secret orphans its old keychain entry unless we
    // clean it up ourselves — but not when `Unchanged` just reused it.
    if let Some(old) = &old_handle {
        if resolved_auth.credential_handle() != Some(old.as_str()) {
            let _ = portus_core::keychain::delete(old);
        }
    }

    let host = Host {
        id,
        name,
        group_id,
        protocol,
        address,
        port,
        username,
        baud_rate,
        auth: resolved_auth,
    };

    if let Some(existing) = config.hosts.iter_mut().find(|h| h.id == id) {
        *existing = host;
    } else {
        config.hosts.push(host);
    }

    portus_core::config::save(&config).map_err(|e| e.to_string())?;
    Ok(config)
}

#[tauri::command]
pub fn delete_host(host_id: Uuid) -> Result<Config, String> {
    let mut config = portus_core::config::load().map_err(|e| e.to_string())?;
    if let Some(pos) = config.hosts.iter().position(|h| h.id == host_id) {
        let removed = config.hosts.remove(pos);
        if let Some(handle) = removed.auth.credential_handle() {
            let _ = portus_core::keychain::delete(handle);
        }
    }
    portus_core::config::save(&config).map_err(|e| e.to_string())?;
    Ok(config)
}

/// Resolves a saved host's stored secret (password or key passphrase) from
/// the keychain, for the frontend to fold into the connect options it
/// sends to `session_open`. `None` if the host has no stored credential.
#[tauri::command]
pub fn resolve_host_secret(host_id: Uuid) -> Result<Option<String>, String> {
    let config = portus_core::config::load().map_err(|e| e.to_string())?;
    let host = config.hosts.iter().find(|h| h.id == host_id).ok_or("host not found")?;
    match host.auth.credential_handle() {
        Some(handle) => portus_core::keychain::retrieve(handle).map(Some).map_err(|e| e.to_string()),
        None => Ok(None),
    }
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
