//! Global application state managed by Tauri's state system.

use std::sync::Mutex;

use crate::domain::probe::ActiveRuntime;

const POISONED_RUNTIME_MUTEX: &str =
    "AppState runtime mutex poisoned (another thread panicked while holding it)";
const POISONED_BOOTSTRAP_MUTEX: &str =
    "AppState firmware_bootstrap mutex poisoned (another thread panicked while holding it)";

/// Application state: the prepared SEGGER runtime (bridge-loaded) and related metadata.
pub struct AppState {
    runtime: Mutex<Option<ActiveRuntime>>,
    firmware_bootstrap_done: Mutex<bool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            runtime: Mutex::new(None),
            firmware_bootstrap_done: Mutex::new(false),
        }
    }

    pub fn set_runtime(&self, rt: ActiveRuntime) {
        *self.runtime.lock().expect(POISONED_RUNTIME_MUTEX) = Some(rt);
    }

    pub fn get_runtime(&self) -> Option<ActiveRuntime> {
        self.runtime.lock().expect(POISONED_RUNTIME_MUTEX).clone()
    }

    /// Returns true exactly once per app session.
    /// Used to run one-time startup maintenance (e.g. firmware ensure).
    pub fn take_firmware_bootstrap_slot(&self) -> bool {
        let mut v = self
            .firmware_bootstrap_done
            .lock()
            .expect(POISONED_BOOTSTRAP_MUTEX);
        if *v {
            return false;
        }
        *v = true;
        true
    }
}