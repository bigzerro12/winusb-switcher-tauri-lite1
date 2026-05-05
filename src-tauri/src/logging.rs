use std::fmt::Display;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static NEXT_OP_ID: AtomicU64 = AtomicU64::new(1);

pub struct OperationLog {
    id: u64,
    name: &'static str,
    started_at: Instant,
}

impl OperationLog {
    pub fn begin(name: &'static str) -> Self {
        let id = NEXT_OP_ID.fetch_add(1, Ordering::Relaxed);
        log::info!("[op:{}] {} start", id, name);
        Self {
            id,
            name,
            started_at: Instant::now(),
        }
    }

    pub fn debug(&self, msg: impl Display) {
        log::debug!("[op:{}] {} {}", self.id, self.name, msg);
    }

    pub fn warn(&self, msg: impl Display) {
        log::warn!("[op:{}] {} {}", self.id, self.name, msg);
    }

    pub fn ok(&self, msg: impl Display) {
        log::info!(
            "[op:{}] {} ok elapsed_ms={} {}",
            self.id,
            self.name,
            self.started_at.elapsed().as_millis(),
            msg
        );
    }

    pub fn fail(&self, msg: impl Display) {
        log::warn!(
            "[op:{}] {} failed elapsed_ms={} {}",
            self.id,
            self.name,
            self.started_at.elapsed().as_millis(),
            msg
        );
    }
}
