//! Compatibility API: telemetry transport is absent in this fork.
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub enum Flush {
    Nothing,
    Later,
    Sent { items: usize, batches: usize },
    Failed(String),
}

/// Even old consent and queued reports cannot trigger a connection.
pub fn flush(_home: &Path, _endpoint: &str) -> Flush {
    Flush::Nothing
}

pub fn delete_remote(_endpoint: &str, _install_id: &str) -> Result<(), String> {
    Err("Telemetry transport was removed from this build".into())
}

pub fn preview(_home: &Path) -> String {
    "[]".into()
}
