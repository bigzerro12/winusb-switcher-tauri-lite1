//! Platform abstraction for process environment management.

/// Update current process PATH so the new dir is usable in this session.
pub fn prepend_to_process_path(dir: &str) {
    let path_key = std::env::vars()
        .find(|(k, _)| k.to_lowercase() == "path")
        .map(|(k, _)| k)
        .unwrap_or_else(|| "PATH".to_string());

    let current = std::env::var(&path_key).unwrap_or_default();
    if !current.to_lowercase().contains(&dir.to_lowercase()) {
        let separator = if cfg!(target_os = "windows") { ";" } else { ":" };
        // Prepend so our bundled J-Link wins over any stale PATH entries.
        std::env::set_var(&path_key, format!("{}{}{}", dir, separator, current));
    }
}

/// After locating a J-Link install directory, apply PATH (all platforms) and Linux shared-library path.
pub fn ensure_jlink_runtime_env(install_dir: &str) {
    prepend_to_process_path(install_dir);
    #[cfg(target_os = "linux")]
    {
        apply_ld_library_path_segger_layout(install_dir);
    }
}

/// Linux: set `LD_LIBRARY_PATH` for a typical SEGGER tree in one shot. Some packages put `*.so`
/// under `x86_64/` or `x86/`; order is **install root first**, then host-relevant arch dirs.
#[cfg(target_os = "linux")]
fn apply_ld_library_path_segger_layout(install_dir: &str) {
    const KEY: &str = "LD_LIBRARY_PATH";
    let base = std::path::Path::new(install_dir);

    let mut front: Vec<String> = Vec::new();
    let push_unique = |v: &mut Vec<String>, s: String| {
        if !v.iter().any(|e| e == &s) {
            v.push(s);
        }
    };

    push_unique(&mut front, install_dir.to_string());

    // Prefer native arch before 32-bit `x86/` on 64-bit hosts (wrong ELF breaks dlopen).
    let sub_order: &[&str] = match std::env::consts::ARCH {
        "x86_64" => &["x86_64", "amd64", "x86"],
        "aarch64" => &["aarch64", "arm64"],
        _ => &["x86_64", "amd64", "x86", "aarch64", "arm64"],
    };
    for sub in sub_order {
        let p = base.join(sub);
        if p.is_dir() {
            push_unique(&mut front, p.to_string_lossy().into_owned());
        }
    }

    let current = std::env::var(KEY).unwrap_or_default();
    for seg in current.split(':') {
        if seg.is_empty() {
            continue;
        }
        push_unique(&mut front, seg.to_string());
    }

    let joined = front.join(":");
    std::env::set_var(KEY, &joined);
    log::info!("[runtime] {}={}", KEY, joined);
}
