#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|a| a == "--jlink-sidecar") {
        std::process::exit(winusb_switcher_lite_lib::run_jlink_sidecar());
    }
    #[cfg(debug_assertions)]
    {
        let _ = ctrlc::set_handler(|| std::process::exit(0));
    }
    winusb_switcher_lite_lib::run();
}