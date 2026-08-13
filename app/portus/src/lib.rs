mod adapter;
mod commands;

use adapter::AppState;

pub fn run() {
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::session_open,
            commands::session_write,
            commands::session_resize,
            commands::session_close,
            commands::get_config,
            commands::save_config,
            commands::list_serial_ports,
        ])
        .run(tauri::generate_context!())
        .expect("error while running portus");
}
