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
├── MyCopilot.exe                     ← Tauri-Launcher (single-file Rust)
├── backend/
│   ├── MyCopilot.Backend.dll         ← C#-Code (self-contained AOT)
│   └── *.dll                         ← .NET-Runtime-Deps
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

| Komponente       | Größe           |
|------------------|-----------------|
| Tauri Rust exe   | ~10 MB          |
| C# AOT Backend   | ~15–30 MB       |
| Node.js embedded | ~30 MB          |
| Copilot CLI+Deps | ~50–100 MB      |
| React-Bundle     | ~2–5 MB         |
| **Gesamt**       | **~110–180 MB** |

## Pfad-Resolution zur Laufzeit

Alle Pfade werden zur Laufzeit **exe-relativ** aufgelöst. NIEMALS
AppData, Registry oder andere systemweite Locations verwenden — sonst
ist die Portabilität gebrochen.

```csharp
using System.IO;
using System.Reflection;

var exeDir = Path.GetDirectoryName(
    Assembly.GetExecutingAssembly().Location)!;

// Node.js Binary
var nodeExe = OperatingSystem.IsWindows()
    ? Path.Combine(exeDir, "node", "node.exe")
    : Path.Combine(exeDir, "node", "node");

// Copilot CLI Entry
var cliEntry = Path.Combine(exeDir, "copilot-cli", "index.js");

// Config & Data
var configPath = Path.Combine(exeDir, "config.json");
var dataDir = Path.Combine(exeDir, "data");
Directory.CreateDirectory(dataDir);  // idempotent

// Subprozess mit korrekten ENV-Vars starten
var psi = new ProcessStartInfo {
    FileName = nodeExe,
    Arguments = $"\"{cliEntry}\"",
    UseShellExecute = false,
    RedirectStandardInput  = true,
    RedirectStandardOutput = true,
    RedirectStandardError  = true,
    CreateNoWindow         = true,
};
psi.Environment["COPILOT_HOME"] = Path.Combine(exeDir, "copilot-cli");
psi.Environment["NODE_PATH"]    = Path.Combine(exeDir, "copilot-cli", "node_modules");
// Wichtig: NICHT PATH ändern, sonst Kollision mit System-Node

var proc = Process.Start(psi)!;
// → Copilot SDK .NET spricht via Stdin/Stdout mit proc (JSON-RPC)
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
3. **C# kompilieren**: `dotnet publish -c Release -r win-x64
   --self-contained true -p:PublishAot=true` → ins Build-Verzeichnis.
4. **Tauri-Build**: `cargo tauri build` → fertiges `MyCopilot.exe`.

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
- Microsoft Learn — `dotnet publish` Self-Contained AOT