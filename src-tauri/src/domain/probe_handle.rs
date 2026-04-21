//! Serializable probe handle for backend routing.
//!
//! The UI currently uses a simple `probe_index` for J-Link only.
//! This handle enables multi-backend routing later without changing internal code shape.

use serde::{Deserialize, Serialize};

use crate::domain::jlink::types::ProbeProvider;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProbeHandle {
    pub provider: ProbeProvider,
    pub probe_index: usize,
}

