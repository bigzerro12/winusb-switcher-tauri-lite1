//! Global application state managed by Tauri's state system.

use std::sync::{Mutex, MutexGuard};

use crate::domain::probe::ActiveRuntime;

const POISONED_RUNTIME_MUTEX: &str =
    "AppState runtime mutex poisoned (another thread panicked while holding it)";
const POISONED_BOOTSTRAP_MUTEX: &str =
    "AppState firmware_bootstrap mutex poisoned (another thread panicked while holding it)";

/// Application state: the prepared SEGGER runtime (bridge-loaded) and related metadata.
pub struct AppState {
    /// Held for process lifetime so the OS releases the lock when we exit (even on crash).
    _instance_lock: std::fs::File,
    runtime: Mutex<Option<ActiveRuntime>>,
    firmware_bootstrap_done: Mutex<bool>,
}

impl AppState {
    pub fn new(instance_lock: std::fs::File) -> Self {
        Self {
            _instance_lock: instance_lock,
            runtime: Mutex::new(None),
            firmware_bootstrap_done: Mutex::new(false),
        }
    }

    pub fn set_runtime(&self, rt: ActiveRuntime) {
        *Self::lock_or_recover(&self.runtime, POISONED_RUNTIME_MUTEX) = Some(rt);
    }

    pub fn get_runtime(&self) -> Option<ActiveRuntime> {
        Self::lock_or_recover(&self.runtime, POISONED_RUNTIME_MUTEX).clone()
    }

    /// Returns true exactly once per app session.
    /// Used to run one-time startup maintenance (e.g. firmware ensure).
    pub fn take_firmware_bootstrap_slot(&self) -> bool {
        let mut v = Self::lock_or_recover(&self.firmware_bootstrap_done, POISONED_BOOTSTRAP_MUTEX);
        if *v {
            return false;
        }
        *v = true;
        true
    }

    fn lock_or_recover<'a, T>(mutex: &'a Mutex<T>, poison_msg: &str) -> MutexGuard<'a, T> {
        match mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                log::warn!("{}", poison_msg);
                poisoned.into_inner()
            }
        }
    }
}
