//! J-Link WinUSB Switcher — Tauri application entry point.

mod bridge_sidecar;
mod bundled_jlink;
mod commands;
mod domain;
mod error;
mod infra;
mod jlink_ffi;
mod logging;
mod paths;
mod platform;
mod single_instance;
mod state;

use state::AppState;
use std::sync::OnceLock;
use tauri::webview::WebviewWindowBuilder;

static SESSION_LOG_STEM: OnceLock<String> = OnceLock::new();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    paths::ensure_app_dirs().unwrap_or_else(|e| {
        eprintln!("create app dirs: {e}");
        std::process::exit(1);
    });

    let lock_path = paths::instance_lock_path();
    let instance_lock = match single_instance::acquire_lock_file(&lock_path) {
        Ok(f) => f,
        Err(()) => {
            single_instance::notify_duplicate_instance();
            std::process::exit(1);
        }
    };

    let session_stem = format!("session-{}", paths::session_timestamp_stem());
    SESSION_LOG_STEM.set(session_stem.clone()).ok();

    let log_dir = paths::log_dir();
    let mut log_builder = tauri_plugin_log::Builder::new()
        .level(log::LevelFilter::Info)
        .level_for("winusb_switcher_lite_lib", log::LevelFilter::Debug)
        .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
        .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
        .max_file_size(256 * 1024 * 1024)
        .clear_targets()
        .target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::Folder {
                path: log_dir,
                file_name: Some(session_stem),
            },
        ));
    #[cfg(debug_assertions)]
    {
        log_builder = log_builder.target(tauri_plugin_log::Target::new(
            tauri_plugin_log::TargetKind::Webview,
        ));
    }

    let builder = tauri::Builder::default()
        .manage(AppState::new(instance_lock))
        .plugin(log_builder.build())
        .setup(|app| {
            let start_ts =
                chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let stem = SESSION_LOG_STEM
                .get()
                .map(String::as_str)
                .unwrap_or("unknown");
            log::info!(
                "[session] started at {} | log file stem: {}",
                start_ts,
                stem
            );

            #[cfg(target_os = "linux")]
            {
                let app_handle = app.handle().clone();
                if let Err(e) =
                    crate::infra::runtime::bundled::ensure_linux_udev_rules_on_startup(&app_handle)
                {
                    log::error!("[bootstrap] Linux udev setup failed at startup: {}", e);
                    return Err(Box::<dyn std::error::Error>::from(e));
                }
            }

            let window_conf = app
                .config()
                .app
                .windows
                .first()
                .ok_or("tauri.conf.json: no windows configured")?;

            WebviewWindowBuilder::from_config(app.handle(), window_conf)
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?
                .data_directory(paths::webview_data_dir())
                .build()
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;

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

    let context = tauri::generate_context!();
    let app = builder
        .build(context)
        .expect("error while building tauri application");
    app.run(|_handle, event| {
        if let tauri::RunEvent::Exit = event {
            let end_ts = chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
            let stem = SESSION_LOG_STEM
                .get()
                .map(String::as_str)
                .unwrap_or("unknown");
            log::info!("[session] ended at {} | log file stem: {}", end_ts, stem);
        }
    });
}

pub fn run_jlink_sidecar() -> i32 {
    bridge_sidecar::run_stdio_sidecar()
}
