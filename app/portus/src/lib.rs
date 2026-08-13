mod adapter;
pub mod commands;
mod rdp_state;
mod sftp_state;

use adapter::AppState;
use rdp_state::RdpState;
use sftp_state::SftpState;

pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .manage(AppState::default())
        .manage(SftpState::default())
        .manage(RdpState::default())
        .invoke_handler(tauri::generate_handler![
            commands::session_open,
            commands::session_write,
            commands::session_resize,
            commands::session_close,
            commands::get_config,
            commands::save_config,
            commands::list_serial_ports,
            commands::save_host,
            commands::delete_host,
            commands::resolve_host_secret,
            commands::sftp_connect,
            commands::sftp_list,
            commands::sftp_read_file,
            commands::sftp_write_file,
            commands::sftp_remove_file,
            commands::sftp_create_dir,
            commands::sftp_remove_dir,
            commands::sftp_disconnect,
            commands::rdp_connect,
            commands::rdp_disconnect,
        ])
        .run(tauri::generate_context!())
        .expect("error while running portus");
}
