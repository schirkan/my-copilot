use thiserror::Error;

/// Fehler beim Copilot-SDK-Support.
///
/// Seit Aktivierung des `bundled-cli`-Features im `github-copilot-sdk`
/// (siehe Cargo.toml) bringt das SDK die CLI-Binary selbst mit --
/// eine manuelle Pfad-Auflösung entfällt. Daher existiert nur noch
/// der generische SDK-Fehler.
#[derive(Debug, Error)]
pub enum ProcessError {
    #[error("Copilot SDK Fehler: {0}")]
    Sdk(String),
}