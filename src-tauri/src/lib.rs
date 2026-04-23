//! WinUSB Switcher Lite — Tauri application entry point.

mod commands;
mod domain;
mod bundled_jlink;
mod error;
mod jlink_ffi;
mod platform;
mod state;
mod infra;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut log_builder = tauri_plugin_log::Builder::new()
        .level(log::LevelFilter::Info)
        .level_for("winusb_switcher_lite_lib", log::LevelFilter::Debug);
    #[cfg(debug_assertions)]
    {
        log_builder = log_builder.target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::Webview,
        ));
    }

    tauri::Builder::default()
        .manage(AppState::new())
        .plugin(log_builder.build())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::prepare_bundled_jlink,
            commands::detect_and_scan,
            commands::scan_probes,
            commands::jlink_exec_command,
            commands::switch_usb_driver,
            commands::get_arch_info,
            commands::get_jlink_diagnostics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}