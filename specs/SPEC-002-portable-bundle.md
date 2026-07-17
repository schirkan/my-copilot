# SPEC-002 — Portable Bundle Layout

**Status:** Planungs-Phase
**Datum:** 2026-07-17
**Bezug:** SPEC-001 § Tech-Entscheidungen → Tauri Bundle-Resources

## Ziel

`My Copilot` läuft als **portabler Ordner** auf Windows 11. Kein MSI,
kein NSIS, keine Registry-Einträge, keine Admin-Rechte. Doppelklick auf
`MyCopilot.exe` startet die App.

## Folder-Layout (final)

```
MyCopilot/                            ← kopierbarer Ordner
├── MyCopilot.exe                     ← Tauri-Launcher + Bridge (single-file Rust)
├── node/
│   └── node.exe                      ← embedded Node.js v22+ portable
├── copilot-cli/
│   ├── package.json
│   ├── index.js                      ← Einstiegspunkt
│   └── node_modules/                 ← via `npm install` im Build
├── config.json                       ← BYOK-Config (DPAPI-verschlüsselt)
└── data/                             ← lokaler State
    ├── sessions/                     ← Chat-History (eine .jsonl pro Session)
    │   └── {session-id}.jsonl
    ├── logs/
    └── cache/
```

**Größe-Schätzung:**

| Komponente                       | Größe           |
|----------------------------------|-----------------|
| Tauri-Rust exe (inkl. Bridge + SDK) | ~25–40 MB   |
| Node.js embedded                 | ~30 MB          |
| Copilot CLI+Deps                 | ~50–100 MB      |
| React-Bundle                     | ~2–5 MB         |
| **Gesamt**                       | **~107–175 MB** |

> Hinweis: Tauri-Rust exe wächst durch Bridge-Logik + Copilot SDK
> Rust von ~10 MB auf ~25–40 MB. Netto-Ersparnis vs. C#-Variante:
> ~5–15 MB (C# AOT-Runtime entfällt komplett).

## Pfad-Resolution zur Laufzeit

Alle Pfade werden zur Laufzeit **exe-relativ** aufgelöst. NIEMALS
AppData, Registry oder andere systemweite Locations verwenden — sonst
ist die Portabilität gebrochen.

```rust
use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

let exe_dir: PathBuf = env::current_exe()?
    .parent()
    .ok_or("no parent dir")?
    .to_path_buf();

// Node.js Binary
let node_exe = exe_dir.join("node").join(
    if cfg!(windows) { "node.exe" } else { "node" }
);

// Copilot CLI Entry
let cli_entry = exe_dir.join("copilot-cli").join("index.js");

// Config & Data
let config_path = exe_dir.join("config.json");
let data_dir    = exe_dir.join("data");
tokio::fs::create_dir_all(&data_dir).await?;  // idempotent

// Subprozess mit korrekten ENV-Vars starten
let mut child = Command::new(&node_exe)
    .arg(&cli_entry)
    .env("COPILOT_HOME", exe_dir.join("copilot-cli"))
    .env("NODE_PATH",    exe_dir.join("copilot-cli").join("node_modules"))
    // Wichtig: NICHT PATH ändern, sonst Kollision mit System-Node
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true)
    .spawn()?;
// → Copilot SDK Rust spricht via Stdin/Stdout mit child (JSON-RPC)
```

## Build-Konfiguration (Tauri)

`tauri.conf.json` (Auszug):
```json
{
  "bundle": {
    "targets": ["app"],
    "resources": {
      "../node/win-x64/node.exe": "node/node.exe",
      "../copilot-cli/index.js": "copilot-cli/",
      "../copilot-cli/package.json": "copilot-cli/",
      "../copilot-cli/node_modules/**": "copilot-cli/node_modules/"
    }
  }
}
```

## Build-Schritte (CI)

1. **Node.js holen**: `curl -L https://nodejs.org/dist/v22.x/node-v22.x-
   win-x64.zip` → extrahieren → `node.exe` ins Build-Verzeichnis.
2. **Copilot CLI installieren**: `npm install @github/copilot-cli`
   → kompletter `node_modules/` ins Build-Verzeichnis.
3. **Tauri-Build**: `cargo tauri build` → fertiges `MyCopilot.exe`
   (App-Shell + Bridge + Copilot SDK Rust in einem Binary).

## Distribution

- **Release-Asset**: ZIP mit komplettem `MyCopilot/`-Ordner.
- **Versionierung**: SemVer (`MAJOR.MINOR.PATCH`), aktuelle Version
  in `Cargo.toml`, `*.csproj`, `package.json` synchron halten.
- **Update-Kanal**: manuelles Re-Download via GitHub-Releases
  (Auto-Update als v2-Feature, siehe Offene Punkte).

## Offene Punkte

- **Code-Signing**: Signieren des `MyCopilot.exe` für SmartScreen-
  Reputation (sonst Warnhinweis beim Erststart).
- **Auto-Update-Mechanismus**: portable Apps können nicht OS-auto-
  updaten — manuelles Re-Download oder Custom-Updater (z. B. Squirrel-
  ähnlich) einbauen?
- **Backup-Strategie**: User kopiert den Folder — was, wenn die Source-
  Folder verschoben/gelöscht wird? Empfehlung: User-Daten in `./data/`
  gehören zum Folder, also mitkopieren.
- **macOS / Linux später**: aktuell nur Windows 11. Andere Plattformen
  würden separate Bundle-Pfade und Node.js-Binaries brauchen.

## Quellen

- Tauri 2.x Bundle-Konfiguration (tauri.app)
- nodejs.org Distribution-Liste
- `tokio::process::Command` — async Subprozess-Management
- Tauri 2 Docs — Sidecar-Pattern