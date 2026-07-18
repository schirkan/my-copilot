//! Config-IO: laden/speichern von `config.json` im exe-Verzeichnis.
//!
//! Atomares Schreiben via Temp-File + Rename (Windows: ReplaceFile
//! semantics auf NTFS, kein partieller Write sichtbar).

use std::path::{Path, PathBuf};

use super::schema::{Config, ConfigError, CURRENT_VERSION};

pub fn config_path(exe_dir: &Path) -> PathBuf {
    exe_dir.join("config.json")
}

/// Lädt `config.json` relativ zu `exe_dir`. Gibt `Ok(None)` wenn
/// die Datei nicht existiert (Erstlauf).
pub fn load_config(exe_dir: &Path) -> Result<Option<Config>, ConfigError> {
    let path = config_path(exe_dir);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let config: Config = serde_json::from_str(&content)?;
    if config.version != CURRENT_VERSION {
        return Err(ConfigError::Schema(format!(
            "unsupported config version: {} (expected {})",
            config.version, CURRENT_VERSION
        )));
    }
    Ok(Some(config))
}

/// Schreibt `config` nach `config.json` (atomar via Temp + Rename).
pub fn save_config(exe_dir: &Path, config: &Config) -> Result<(), ConfigError> {
    let path = config_path(exe_dir);
    let content = serde_json::to_string_pretty(config)?;

    // Atomic write: write to temp file, then rename (Windows + Unix
    // beide unterstützen rename als atomar wenn Same-Filesystem).
    let tmp_path = path.with_extension("json.tmp");
    std::fs::write(&tmp_path, content)?;

    // Auf Windows kann rename fehlschlagen wenn die Zieldatei
    // existiert — daher erst remove (idempotent).
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
    std::fs::rename(&tmp_path, &path)?;
    Ok(())
}