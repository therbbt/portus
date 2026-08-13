use bytes::Bytes;
use tauri::{AppHandle, State};

use portus_core::config::Config;

use crate::adapter::{AppState, SessionCommand};

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
