//! J-Link WinUSB Switcher — Tauri application entry point.

mod bridge_sidecar;
mod bundled_jlink;
mod commands;
mod domain;
mod error;
mod infra;
mod jlink_ffi;
mod logging;
mod platform;
mod state;

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

    let builder = tauri::Builder::default()
        .manage(AppState::new())
        .plugin(log_builder.build())
        .setup(|_app| {
            #[cfg(target_os = "linux")]
            {
                let app_handle = _app.handle().clone();
                if let Err(e) =
                    crate::infra::runtime::bundled::ensure_linux_udev_rules_on_startup(&app_handle)
                {
                    log::error!("[bootstrap] Linux udev setup failed at startup: {}", e);
                    return Err(Box::<dyn std::error::Error>::from(e));
                }
            }
            Ok(())
        });

    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::prepare_bundled_jlink,
        commands::detect_and_scan,
        commands::scan_probes,
        commands::switch_usb_driver,
        commands::get_arch_info,
        commands::get_jlink_diagnostics,
    ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn run_jlink_sidecar() -> i32 {
    bridge_sidecar::run_stdio_sidecar()
}
