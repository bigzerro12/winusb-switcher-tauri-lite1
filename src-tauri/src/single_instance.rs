//! One main GUI process at a time (sidecar mode skips this).

use fs2::FileExt;
use std::fs::OpenOptions;
use std::path::Path;

pub fn acquire_lock_file(lock_path: &Path) -> Result<std::fs::File, ()> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .map_err(|_| ())?;
    file.try_lock_exclusive().map_err(|_| ())?;
    Ok(file)
}

pub fn notify_duplicate_instance() {
    let msg = "Another instance of J-Link WinUSB Switcher is already running.";
    eprintln!("{msg}");

    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONWARNING, MB_OK};

        fn nul_terminated_wide(s: &str) -> Vec<u16> {
            OsStr::new(s)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        }

        let title = nul_terminated_wide("J-Link WinUSB Switcher");
        let body = nul_terminated_wide(msg);
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                body.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONWARNING,
            );
        }
    }

    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("zenity")
            .args([
                "--error",
                "--no-wrap",
                "--title=J-Link WinUSB Switcher",
                &format!("--text={msg}"),
            ])
            .spawn();
        let _ = std::process::Command::new("kdialog")
            .args(["--error", msg])
            .spawn();
    }
}
