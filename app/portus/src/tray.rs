//! System tray icon plus a global "Alt+T" hotkey that toggles the main
//! window's visibility, and turns the window's own close button into the
//! same hide-to-tray action. Once this is active, the tray menu's "Quit"
//! item is the only way to actually exit — closing the window just hides
//! it, matching FlashPad's tray behavior.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::plugin::TauriPlugin;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent, Wry};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/tray-icon.png");
const HOTKEY: &str = "Alt+T";

/// Tracks whether the main window is currently shown, so the hotkey/tray
/// can toggle instantly without re-querying the window — a window's own
/// `is_visible()` has been observed to lag behind reality on some Linux/
/// Wayland setups, which would make a toggle occasionally need a second
/// press to actually take effect.
static WINDOW_SHOWN: AtomicBool = AtomicBool::new(true);

fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
        WINDOW_SHOWN.store(false, Ordering::SeqCst);
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        WINDOW_SHOWN.store(true, Ordering::SeqCst);
    }
}

fn toggle_main_window(app: &AppHandle) {
    if WINDOW_SHOWN.load(Ordering::SeqCst) {
        hide_main_window(app);
    } else {
        show_main_window(app);
    }
}

/// The global-shortcut plugin, wired to toggle the window — register this
/// with `Builder::plugin` before the app runs.
pub fn global_shortcut_plugin() -> TauriPlugin<Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed {
                toggle_main_window(app);
            }
        })
        .build()
}

/// Registers the hotkey with the OS, builds the tray icon, and rewires the
/// main window's close button — call once from `Builder::setup`.
pub fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    app.global_shortcut().register(HOTKEY)?;

    let open_item = MenuItem::with_id(app, "open", "Open Portus", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let tray_menu = Menu::with_items(app, &[&open_item, &quit_item])?;

    let tray_icon = Image::from_bytes(TRAY_ICON_BYTES).expect("failed to decode tray icon");
    TrayIconBuilder::new()
        .icon(tray_icon)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    if let Some(window) = app.get_webview_window("main") {
        let app_handle = app.handle().clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                hide_main_window(&app_handle);
            }
        });
    }

    Ok(())
}
