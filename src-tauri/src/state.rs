//! Global application state managed by Tauri's state system.

use std::sync::Mutex;

use crate::infra::runtime::bundled::JLinkRuntime;

/// Application state: the prepared SEGGER runtime (bridge-loaded) and related metadata.
pub struct AppState {
    runtime: Mutex<Option<JLinkRuntime>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            runtime: Mutex::new(None),
        }
    }

    pub fn set_runtime(&self, rt: JLinkRuntime) {
        *self.runtime.lock().unwrap() = Some(rt);
    }

    pub fn get_runtime(&self) -> Option<JLinkRuntime> {
        self.runtime.lock().unwrap().clone()
    }
}