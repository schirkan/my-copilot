use std::path::PathBuf;

use thiserror::Error;

/// Fehler beim Copilot-SDK-Support.
#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Copilot SDK Fehler: {0}")]
    Sdk(String),
    #[error("Copilot-Binary nicht gefunden: {0}")]
    BinaryNotFound(String),
}

pub fn resolve_copilot_binary_path(exe_dir: &PathBuf) -> Result<PathBuf, ProcessError> {
    let exe_name = "copilot-x86_64-pc-windows-msvc.exe";

    let candidates = [
        exe_dir.join("binaries").join(exe_name),
        exe_dir
            .parent()
            .map(|p| p.join("binaries").join(exe_name))
            .unwrap_or_default(),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries").join(exe_name),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Ok(candidate.clone());
        }
    }

    Err(ProcessError::BinaryNotFound(
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
    ))
}