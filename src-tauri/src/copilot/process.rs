//! Subprozess-Management für die Copilot CLI.
//!
//! Verantwortlich für:
//! - Spawn von `node/node.exe copilot-cli/index.js`
//! - Stdin/Stdout/Stderr-Pipes (kein Port, kein HTTP — siehe DECISIONS.md)
//! - kill_on_drop für sauberen Lifecycle
//! - Async stderr → log-Warnung
//!
//! Siehe SPEC-004 § Subprozess-Management.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// Fehler, die beim Subprozess-Management auftreten können.
#[derive(Debug)]
pub enum ProcessError {
    Io(std::io::Error),
    NodeBinaryMissing(PathBuf),
    CliEntryMissing(PathBuf),
    NoStdin,
    NoStdout,
    StdinWrite(String),
    StdoutTake(String),
    ReadyTimeout,
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "I/O-Fehler: {}", e),
            Self::NodeBinaryMissing(p) => write!(f, "node-Binary nicht gefunden: {:?}", p),
            Self::CliEntryMissing(p) => write!(f, "CLI-Entry nicht gefunden: {:?}", p),
            Self::NoStdin => write!(f, "child stdin nicht verfügbar"),
            Self::NoStdout => write!(f, "child stdout nicht verfügbar"),
            Self::StdinWrite(e) => write!(f, "stdin write: {}", e),
            Self::StdoutTake(e) => write!(f, "stdout take: {}", e),
            Self::ReadyTimeout => write!(f, "Ready-Timeout (CLI hat nicht rechtzeitig geantwortet)"),
        }
    }
}

impl std::error::Error for ProcessError {}

impl From<std::io::Error> for ProcessError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Wrapper um den laufenden CLI-Subprozess.
///
/// Felder `child`, `stdin`, `stdout` werden nach außen hin nicht direkt
/// exposed — der Zugriff läuft über [`CopilotBridge`].
pub struct CopilotCliProcess {
    pub(crate) child: Child,
    pub(crate) stdin: ChildStdin,
    pub(crate) stdout: Option<ChildStdout>,
    pub(crate) node_exe: PathBuf,
    pub(crate) cli_entry: PathBuf,
}

impl CopilotCliProcess {
    /// Startet den Subprozess: `node/node.exe copilot-cli/index.js`.
    ///
    /// Pfade werden **exe-relativ** aufgelöst (siehe SPEC-002 §
    /// Pfad-Resolution). Im **Release-Bundle** liegt ein embedded
    /// Node.js unter `<exe_dir>/node/node.exe` und die
    /// Copilot-CLI unter `<exe_dir>/copilot-cli/index.js`. Im
    /// **Dev-Modus** (`cargo run`) gibt es den Bundle-Pfad nicht —
    /// wir fallen auf den Source-Pfad `src-tauri/copilot-cli/`
    /// zurück (per `CARGO_MANIFEST_DIR`) und auf System-Node statt
    /// embedded Node.
    ///
    /// ENV-Variablen `COPILOT_HOME` und `NODE_PATH` werden explizit
    /// gesetzt. PATH wird NICHT überschrieben (Kollision mit
    /// System-Node vermeiden).
    pub fn start(exe_dir: &Path) -> Result<Self, ProcessError> {
        // copilot-cli-Verzeichnis wählen:
        //   1) <exe_dir>/copilot-cli    (Release-Bundle)
        //   2) <src-tauri>/copilot-cli  (Dev-Modus, via CARGO_MANIFEST_DIR)
        let cli_root = resolve_cli_root(exe_dir)?;
        let cli_entry = cli_root.join("index.js");

        if !cli_entry.exists() {
            return Err(ProcessError::CliEntryMissing(cli_entry));
        }

        // Node-Binary wählen: zuerst embedded, dann System-Node.
        let node_exe = resolve_node_binary(exe_dir)?;
        log::info!("Verwende Node-Binary: {:?}", node_exe);
        log::info!("Verwende CLI-Root: {:?}", cli_root);

        let mut child = Command::new(&node_exe)
            .arg(&cli_entry)
            .env("COPILOT_HOME", &cli_root)
            .env("NODE_PATH", cli_root.join("node_modules"))
            // Wichtig: PATH nicht ändern (Kollision mit System-Node)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdin = child.stdin.take().ok_or(ProcessError::NoStdin)?;
        let stdout = child.stdout.take().ok_or(ProcessError::NoStdout)?;

        // stderr → async Log-Handler
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    log::warn!("[copilot-cli] {}", line);
                }
            });
        }

        log::info!(
            "Copilot CLI gestartet: node={:?}, entry={:?}",
            node_exe,
            cli_entry
        );

        Ok(Self {
            child,
            stdin,
            stdout: Some(stdout),
            node_exe,
            cli_entry,
        })
    }

    /// Killt den Subprozess explizit (kill_on_drop=true erledigt das
    /// auch automatisch beim Drop).
    pub async fn kill(&mut self) -> Result<(), ProcessError> {
        self.child.kill().await?;
        log::info!("Copilot CLI-Subprozess gekillt");
        Ok(())
    }

    /// Prüft, ob der Subprozess noch läuft.
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, ProcessError> {
        Ok(self.child.try_wait()?)
    }

    /// Warte auf ein Ready-Signal der CLI (erste JSON-RPC-Nachricht auf
    /// stdout, die kein Stream-Chunk ist). Vereinfachte v1-Implementierung:
    /// liest eine Zeile mit Timeout.
    pub async fn wait_for_ready(&mut self, _timeout: Duration) -> Result<(), ProcessError> {
        // v1: kein expliziter Ready-Handshake. Wir prüfen nur, dass der
        // Prozess nicht sofort stirbt. Health-Check passiert implizit
        // über den ersten chat_streaming-Call (wenn das Senden scheitert,
        // bekommen wir einen Error).
        //
        // TODO v2: expliziter Ready-Handshake via JSON-RPC `initialize`-
        // Methode nach SPEC-004 § Subprozess-Management.
        match self.child.try_wait()? {
            Some(status) => {
                log::error!("Copilot CLI sofort gestorben: {:?}", status);
                Err(ProcessError::ReadyTimeout)
            }
            None => Ok(()),
        }
    }

    pub fn node_exe(&self) -> &Path { &self.node_exe }
    pub fn cli_entry(&self) -> &Path { &self.cli_entry }
}

/// Wählt das zu verwendende Node.js-Binary.
///
/// Reihenfolge:
/// 1. `<exe_dir>/node/node.exe` (Release-Bundle, embedded Node)
/// 2. `node` auf PATH (Dev-Modus auf Windows: findet das System-Node)
/// 3. Bekannte absolute Pfade (`C:\Program Files\nodejs\node.exe`)
fn resolve_node_binary(exe_dir: &Path) -> Result<PathBuf, ProcessError> {
    let bundled = exe_dir.join("node").join(
        if cfg!(windows) { "node.exe" } else { "node" }
    );
    if bundled.exists() {
        return Ok(bundled);
    }

    // System-Node über PATH (Windows nutzt `where`, Unix `which`)
    let probe = if cfg!(windows) { "where" } else { "which" };
    let output = std::process::Command::new(probe)
        .arg("node")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            // erste nicht-leere Zeile
            s.lines().find(|l| !l.trim().is_empty()).map(|s| s.trim().to_string())
        });

    if let Some(p) = output {
        return Ok(PathBuf::from(p));
    }

    // Bekannte Windows-Defaults (Last-Resort-Fallback)
    if cfg!(windows) {
        for candidate in [
            r"C:\Program Files\nodejs\node.exe",
            r"C:\Program Files (x86)\nodejs\node.exe",
        ] {
            let p = PathBuf::from(candidate);
            if p.exists() {
                return Ok(p);
            }
        }
    }

    Err(ProcessError::NodeBinaryMissing(bundled))
}

/// Wählt das Verzeichnis, in dem die Copilot-CLI liegt.
///
/// Reihenfolge:
/// 1. `<exe_dir>/copilot-cli` (Release-Bundle)
/// 2. `<CARGO_MANIFEST_DIR>/copilot-cli` (Dev-Modus — Cargo setzt
///    diese Env-Var beim Build, sie zeigt auf das Tauri-Crate-Root
///    `src-tauri/`).
fn resolve_cli_root(exe_dir: &Path) -> Result<PathBuf, ProcessError> {
    let bundled = exe_dir.join("copilot-cli");
    if bundled.join("index.js").exists() {
        return Ok(bundled);
    }

    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let dev = PathBuf::from(manifest).join("copilot-cli");
        if dev.join("index.js").exists() {
            return Ok(dev);
        }
    }

    // Last-Resort: gib den erwarteten Bundle-Pfad zurück, damit
    // der Aufrufer eine aussagekräftige Fehlermeldung bekommt.
    Err(ProcessError::CliEntryMissing(bundled.join("index.js")))
}