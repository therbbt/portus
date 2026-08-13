use bytes::Bytes;
use serde::Deserialize;
use tauri::{AppHandle, State};
use uuid::Uuid;

use portus_core::config::{AuthMethod, Config, Host};
use portus_core::session::Protocol;
use portus_sftp::DirEntry;
use portus_ssh::SshConnectOptions;

use crate::adapter::{AppState, SessionCommand};
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
    Password { password: String },
    PrivateKey { path: String, passphrase: Option<String> },
}

fn resolve_auth(auth: AuthInput) -> Result<AuthMethod, String> {
    match auth {
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

/// Creates or fully replaces a saved host. Always resolves a fresh
/// `AuthMethod` from `auth` — there's no "keep the existing secret"
/// affordance yet, since nothing in the UI edits a saved host's credential
/// independently of retyping it.
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

    // Replacing a host's secret orphans its old keychain entry unless we
    // clean it up ourselves.
    if let Some(old_handle) = config
        .hosts
        .iter()
        .find(|h| h.id == id)
        .and_then(|h| h.auth.credential_handle())
    {
        let _ = portus_core::keychain::delete(old_handle);
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
        auth: resolve_auth(auth)?,
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
