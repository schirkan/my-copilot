# My Copilot — Projekt-Übersicht

> Projekt-Modus aktiv für neues dediziertes Projekt **My Copilot**.
> Workspace-Pfad: `projects/my-copilot/`
> Erstellt: 2026-07-17
> Vorgeschichte: Ausgelagert aus `projects/ai-ideas/specs/IDEA-006-ai-agent-runtime.md`
> und der Folge-Diskussion (16.–17.07.2026) im AI-Ideas-Topic.

## Zweck

Portable Desktop-App für KI-Agent-Workflows. Eigener OpenAI-kompatibler
Endpoint (BYOK), Multi-Provider-fähig, läuft auf Windows 11 ohne
Installation — weder Node.js noch Python noch .NET-Framework wird vom
End-User vorausgesetzt.

Vollständige Architektur-Doku: `specs/SPEC-001-architecture-stack.md`.

## Tech-Stack (Kurzfassung)

| Komponente         | Technologie                | Sprache                  |
| ------------------ | -------------------------- | ------------------------ |
| App-Shell + Bridge | Tauri 2 + Copilot SDK Rust | Rust                     |
| Runtime            | Copilot CLI Runtime        | Node.js / native runtime |
| Frontend           | React + CopilotKit UI      | TypeScript / JSX         |
| LLM-Provider       | OpenAI-kompatibel          | n/a                      |

Detaillierte Aufschlüsselung pro Layer in
`specs/SPEC-001-architecture-stack.md`.

## Aktueller Status

- **2026-07-17**: Projekt angelegt — Folder, PROJECT.md, 5 Specs.
- **2026-07-17**: Git-Repo eingerichtet (siehe `## Git`) — public auf
  GitHub unter `schirkan/my-copilot`.
- **2026-07-17**: MIT-Lizenz hinzugefügt (`LICENSE`).
- **2026-07-17**: 5 Decisions dokumentiert in `DECISIONS.md`
  (Node.js Build+Runtime, BYOK-only, kein Update, ohne Signing,
  GitHub Releases).
- **2026-07-17**: 6. Decision: Persistenz-Format JSONL.
  `SPEC-004` § Persistenz von SQLite auf JSONL umgeschrieben.
- **2026-07-17**: Neue `SPEC-006 — Config Dialog` für API Key, Base URL,
  System Prompt und MCP Servers.
- **2026-07-17**: Architektur-Verschlankung — C#-Backend ersatzlos
  gestrichen, Tauri-Rust übernimmt Bridge-Logik (Copilot SDK Rust).
  2 statt 3 Prozesse, kein HTTP-Port für IPC (Stdin/Stdout-Pipes),
  ~5–15 MB Bundle-Ersparnis netto. SPEC-001/002/004/005/006 +
  DECISIONS.md umgeschrieben.
- **2026-07-17 (M1 abgeschlossen)**: Tauri-Skeleton aufgesetzt.
  15 Source-Files (Cargo.toml, build.rs, tauri.conf.json, main.rs,
  lib.rs, capabilities/default.json, package.json, src/index.html +
  main.tsx + App.tsx + App.css + vite.config.ts + tsconfig.json +
  tsconfig.node.json + index.css). `npm install` (619 packages),
  `npx tauri icon` für Windows/iOS/Android/macOS-Varianten,
  `cargo check` erfolgreich (Finished in 9.44s). Workboard-Karte
  #1 läuft jetzt auf `complete`. Nächste Schritte in Karten #2–#9.
- **2026-07-17 (M2 abgeschlossen)**: Tauri-Rust CLI-Bridge implementiert.
  `src-tauri/src/copilot/{mod,process,bridge}.rs` neu angelegt
  (Subprozess-Management via `tokio::process::Command` + `Stdio::piped()` +
  `kill_on_drop` + async stderr-Log, JSON-RPC-2.0-Bridge mit
  mpsc-Channel-Streaming-Pattern). Cargo.toml erweitert um
  `tokio` (full), `tokio-stream`, `log`, `env_logger`. `cargo check`
  exit 0 (2.19s nach erstem Compile). **Kein Port, kein HTTP** — Pipes
  only (siehe DECISIONS.md § Architektur-Verschlankung).
- **2026-07-17 (M3 abgeschlossen)**: Tauri-IPC-API definiert. 7 Tauri-
  Commands (`chat_send`, `chat_cancel`, `config_get`, `config_set`,
  `config_test`, `process_health`, `process_restart`) +
  `AppState`-Struct (`exe_dir`, `config: Mutex<Option<ByokConfig>>`,
  `bridge: Mutex<Option<CopilotBridge>>`, `healthy: AtomicBool`)
  + `ConfigDto` (Frontend-DTO mit systemPrompt + mcpServers bereits
  enthalten). Cargo.toml erweitert um `reqwest` (json + rustls-tls).
  `cargo check` exit 0 (2.69s nach Compile-Cache). lib.rs registriert
  alle 7 Commands via `tauri::generate_handler![]` + managet AppState
  im setup-Hook.
- **2026-07-18 (M4 abgeschlossen — Klartext)**: Config-Management
  mit Klartext-`apiKey` in `config.json` (Martins Direktive „Keep it
  simple"). `dpapi.rs` komplett entfernt (trivialer Passthrough wäre
  irreführend). Schema 1 inkl. `systemPrompt` + `mcpServers`.
  Atomares `load_config`/`save_config` (Temp + Replace). ConfigError
  ohne DPAPI-Variants. `commands/config.rs`: `config_set` ohne
  Encryption. `lib.rs`-Setup lädt `config.json` ohne Decryption.
  Cargo.toml: KEINE `base64` oder `windows`-Crate. `cargo check`
  exit 0 (0.77s nach Compile-Cache). DECISIONS.md: neue Decision
  „Config-Storage: v1 Klartext, v2 DPAPI-TODO".
- **2026-07-18 (M5 abgeschlossen — Config-Dialog UI)**:
  `src/ConfigDialog.tsx` (Modal mit 3 Tabs Connection/Behavior/Tools),
  `src/ConfigDialog.css` (Dark-Theme-Styling), `src/App.tsx`
  erweitert (`useEffect` config_get beim Mount, Settings-Button).
  `tsc -p src/tsconfig.json` exit 0. Doku in PROJECT.md.
- **2026-07-18 (M6 abgeschlossen — JSONL-Chat-History)**:
  Persistence-Layer `src-tauri/src/persistence/{mod,session,append,
  read}.rs`. Eine Datei pro Session in
  `data/sessions/{session-id}.jsonl`, append-only via
  `tokio::fs::OpenOptions::append()`. `Message` + `SessionMeta`
  Structs, `load_session` + `list_sessions` + `delete_session`. 4
  neue Tauri-Commands in `src-tauri/src/commands/history.rs`. `chat_send`
  integriert: persistiert User-Message vor Streaming und
  Assistant-Message danach (gleiche UUID-v4-`session_id`). Cargo.toml
  erweitert um `chrono = "0.4"` + `uuid = "1" (v4)`. `cargo check`
  exit 0 (1.98s nach Compile-Cache).
- **2026-07-18 (M7 abgeschlossen — Chat-UI)**:
  `src/ChatWindow.tsx` mit lokaler Message-State-Logik, Sidebar für
  Session-Liste, InputBox und Tauri-IPC-Command `chat_send`. CopilotKit
  bleibt als UI-Option in der Architektur, aber nicht als Runtime-
  Provider im Frontend. `src/ChatWindow.css` Dark-Theme-Styling
  (Sidebar 240px, Messages links/rechts-bündig, InputBox unten,
  auto-scroll zur neuesten Message). `src/App.tsx` zeigt
  `<ChatWindow />`, sobald Config geladen ist.

- **2026-07-31 (M10 abgeschlossen — Streaming-Architektur v2)**:
  Komplette Refaktorierung der Chat-Pipeline von non-streaming
  (Request/Response) auf persistente-Client + Event-basierte Streams.
  Konkret:
  - **Persistente `CopilotBridge`**: `AppState.bridge` haelt den
    SDK-Client (und damit CLI-Subprozess) fuer die App-Lifetime;
    `ensure_bridge()` lazy-init oder Recreation bei Config-Mismatch.
  - **Per-Message `Session`**: `bridge.create_session()` liefert
    frische Session pro Request, sauber lifecyclebar fuer
    `abort()` / `disconnect()`.
  - **Echtzeit-Streaming**: `Session::subscribe()` + `with_streaming(true)`
    routen `assistant.message_delta`-Token-Events durch. Drei
    neue Tauri-Events: `chat_chunk` (pro Delta), `chat_done`
    (Antwort fertig), `chat_error` (Fehler).
  - **Stable Session-IDs**: `chat_send` akzeptiert optional
    `session_id` und returnt `{session_id, request_id}`. Frontend
    trackt `currentSessionId`, Folge-Messages haengen an dieselbe
    Session an (eine JSONL pro Session, nicht pro Message).
  - **Synchronous Cancellation**: `chat_cancel(request_id)` ruft
    `session.abort()` auf der aktiven Session (gehalten in
    `AppState.active_session` als `Arc<tokio::sync::Mutex<Session>>`).
  - **Race-freie Frontend-Bubble**: User-Message und Assistant-Bubble
    werden erst NACH der `chat_send`-Response gerendert (mit der
    Server-`request_id` als Marker), damit `chat_chunk`-Events,
    die parallel ankommen, immer die richtige Bubble finden.
  - **Endpoint-Normalisierung**: `dedupe_v1_suffix()` (statt
    `strip_v1_suffix()`) -- reduziert nur doppelte `/v1/v1`-Suffixe,
    laesst einzelne `/v1` unangetastet (viele OpenAI-kompatible
    Provider erwarten `/v1` als Teil der Basis-URL, z. B. MiniMax M3).
    Unit-Tests in `bridge.rs::tests`.
  - **Tauri-Event-Naming**: snake_case (`chat_chunk`, `chat_done`,
    `chat_error`) statt dotted (`chat.chunk` etc.), weil Tauri 2
    nur `[a-zA-Z0-9-/:_]` in Event-Namen zulaesst.
  - **CI-Status**: manuell in Dev getestet, Streaming end-to-end
    funktioniert mit MiniMax M3 (484 Token-Deltas pro Sample-Prompt,
    accumulated_len=947 Bytes, `chat_done` mit full-content).
  - Geänderte Dateien: `src-tauri/src/copilot/bridge.rs`,
    `src-tauri/src/state.rs`, `src-tauri/src/commands/chat.rs`,
    `src-tauri/src/commands/process.rs`, `src-tauri/src/lib.rs`,
    `src/ChatWindow.tsx`, `src/ChatWindow.css`,
    `specs/SPEC-004-bridge-tauri-rust.md`.

- **2026-07-26 (Runtime-Migration abgeschlossen)**:
  Der manuelle ACP-Handshake wurde durch das offizielle Rust-SDK
  (`github-copilot-sdk`) ersetzt. Die Tauri-Rust-Bridge erstellt
  Sessions über `Client::create_session(...)` + `ProviderConfig` und
  sendet Nachrichten via `send_and_wait(...)`. BYOK mit MiniMax läuft
  erfolgreich end-to-end (`ping` → `Pong!`).
- **2026-07-19 (CI-Pipeline grün — rc1 bis rc17)**:
  13 Pipeline-Fixes nötig (Commits `30e6208`–`33be1fb` Fix #6–#14) nach
  initialem Workflow-Commit aus M9. Hauptprobleme: PowerShell-`exec` +
  `working-directory`-Cwd-Propagation, Vite-Cwd-Loss via npm-Shims,
  Binary-Name = Cargo-Paketname (`my-copilot.exe`, nicht `productName`),
  GitHub-Actions-v4-Deprecation. **rc14–rc17 grün**, alle 14 Steps success,
  GitHub-Release + ZIP-Bundle automatisch erstellt.
- **2026-07-19 (npm-Update-Block — Fix #15–#19)**:
  Lokale Updates mit Einzel-Commits pro Paket, ein Push mit rc17-Tag am
  Ende (Direktive Martin #5610): zustand 4.5.7→5.0.14 (`d9abaea`),
  React 18→19 + react-dom + types (`a97e44f`), Vite 5→8 + plugin-react
  4→6 + Oxc-Minifier (`3d7e7e1`), TypeScript 5→7 (`9945e81`),
  Spec-Ranges verschärft (`f4526d6`). Alle Updates semver-konform,
  lokaler Build grün (9141 Modules, ~4s).
- **2026-07-20 (CI in PROJECT.md dokumentiert + v0.1.0 stable)**:
  CI-Sektion in PROJECT.md hinzugefügt. Nach 8 grünen rc-Runden
  (rc10–rc17) wird das erste stable Release `v0.1.0` getaggt.
- **2026-07-29 (FF-Merge origin/main + CI-Doku re-sync)**:
  8 Commits von origin/main gemergt (FF, ab724a7..39ed573),
  inkl. `c5873d7` (Rust-Copilot-SDK-Migration via `Client::create_session(...)` +
  `ProviderConfig` + `send_and_wait(...)`), `ba6e31e` (CopilotKit-Runtime-Errors
  + ErrorBoundary), `fbc6b38` (`/info`-Stub vom Vite-Dev-Server), `d5bb012`
  (`appendMessage`-Bugfix), `9142224` (Dev-Mode Node-Resolution + BYOK-JSON-RPC-
  Bridge-Shim), `61b1e50` (Vite-Config-Pfad im Dev-Script). CI-Sektion in
  PROJECT.md an neue Architektur angepasst: Pipeline-Step 7 als non-blocking
  markiert, Build-Output-Schema auf Rust-SDK ohne Node-Subprozess aktualisiert,
  v0.1.0 als ✅ grün markiert, v0.1.0-rc18 als ausstehend vorgemerkt.

## Git

| Feld                    | Wert                                                     |
| ----------------------- | -------------------------------------------------------- |
| **Repo-Typ**            | GitHub (public)                                          |
| **Pfad / URL**          | `https://github.com/schirkan/my-copilot`                 |
| **Lokaler Pfad**        | `C:\Users\Admin\.openclaw\workspace\projects\my-copilot` |
| **Remote(s)**           | `origin` → `https://github.com/schirkan/my-copilot.git`  |
| **Default-Branch**      | `main`                                                   |
| **Eingerichtet am**     | 2026-07-17                                               |
| **`.gitignore`-Status** | vorhanden                                                |
| **Lizenz**              | MIT (siehe `LICENSE`)                                    |

> Hinweis: Der OpenClaw-Workspace-Root (`C:\Users\Admin\.openclaw\workspace`)
> ist ein separates Git-Repo. `projects/my-copilot/` ist dort **nicht**
> getrackt — eigenständiges Repo.

## CI / GitHub Actions

**Workflow:** `.github/workflows/release.yml`
**Triggers:** `push: tags: 'v*'`, `workflow_dispatch`
**Runner:** `windows-latest` (Windows 11, kein Cross-Build Linux → Windows)

### Pipeline-Schritte (14)

| #   | Step                                   | Action                                                                                             |
| --- | -------------------------------------- | -------------------------------------------------------------------------------------------------- |
| 1   | Set up job                             | Runner-Init                                                                                        |
| 2   | Checkout                               | `actions/checkout@v5`                                                                              |
| 3   | Setup Node.js                          | `actions/setup-node@v5`, Node 22                                                                   |
| 4   | Setup Rust                             | `dtolnay/rust-toolchain@stable`                                                                    |
| 5   | Cache cargo registry                   | `Swatinem/rust-cache@v2`                                                                           |
| 6   | Install npm dependencies (frontend)    | `npm ci`                                                                                           |
| 7   | Install npm dependencies (Copilot CLI) | `npm install @github/copilot-cli` (bash, non-blocking — obsolet seit Rust-SDK-Migration `c5873d7`) |
| 8   | Install tauri-cli                      | via `npm ci` (Schritt 6) — spart ~5–10 Min/Run ggü. `cargo install`                                |
| 9   | Build frontend (tsc + vite)            | `cd "$GITHUB_WORKSPACE" && npm run build` (bash, cwd-agnostisch)                                   |
| 10  | Build Tauri app (no-bundle)            | `cd "$GITHUB_WORKSPACE/src-tauri" && npm exec -- tauri build --no-bundle` (bash)                   |
| 11  | Locate Tauri build output              | `Get-ChildItem -Filter "my-copilot.exe"` (pwsh)                                                    |
| 12  | Assemble portable bundle               | Kopiert Binary + Cargo-Deps in `bin/` (pwsh)                                                       |
| 13  | Upload artifact                        | `actions/upload-artifact@v5`                                                                       |
| 14  | Create GitHub Release                  | `softprops/action-gh-release@v3` (ZIP)                                                             |

### Versioning-Strategie

- **`vX.Y.Z`** — Stable Release (Production-ready)
- **`vX.Y.Z-rcN`** — Release Candidate (N inkrementiert pro Push)

Workflow wird per `git push origin vX.Y.Z-rcN` oder `vX.Y.Z` getriggert.
**Tags dürfen nicht verschoben werden** — GitHub Releases zeigen sonst
auf einen falschen Commit. Bei Fehlern: neuen rc-Tag pushen, alten
nicht löschen (bleibt als History in den Releases).

### Build-Output (ZIP-Bundle)

> **Stand 2026-07-26:** Bundle-Layout durch Rust-Copilot-SDK-Migration (`c5873d7`) vereinfacht — Tauri-Rust-Binary ist self-contained, kein Node.js-Subprozess mehr nötig.

```
MyCopilot-portable-v0.1.0.zip
├── MyCopilot.exe              ← Tauri-Rust-Binary (statisch gelinkt, ~30–50 MB, mit einkompiliertem `github-copilot-sdk`)
├── README.txt                 ← Erstlauf-Dialog mit Endpoint- und API-Key-Hinweisen (auto-generiert im Assemble-Step)
├── node/                      ← (nur wenn vorhanden, obsolet seit `c5873d7`)
└── copilot-cli/               ← (nur wenn vorhanden, obsolet seit `c5873d7`)
```

Gesamtgröße: **~40–60 MB** (kleiner als v0.1.0-Variante mit Node-Subprozess-Bundle). Kein Code-Signing in v1 (v3-Feature). Persistenz: `config.json` + `data/sessions/{session-id}.jsonl` werden im exe-Verzeichnis angelegt.

**v0.1.0-Bundle (vor Rust-Migration, archiviert):**
```
my-copilot-v0.1.0.zip
└── bin/
    ├── my-copilot.exe          ← Tauri-Binary
    ├── copilot-cli/            ← Node.js-Subprozess (obsolet seit `c5873d7`)
    └── node/                   ← Embedded Node.js (obsolet seit `c5873d7`)
```

### Lessons Learned (Pipeline-Fixes #1–#14)

13 Iterationen von rc1 bis rc14 nötig. Hauptlessons:

| #   | Lesson                                                                          | Fix                                                                    |
| --- | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| 1   | PowerShell-`Set-Location` propagiert nicht zu `[Environment]::CurrentDirectory` | `$GITHUB_WORKSPACE` + explizite `cd` in `run:`                         |
| 2   | `working-directory` + PowerShell-`exec` ist fragil                              | `shell: bash` für Steps mit Cwd-Manipulation                           |
| 3   | Vite-Cwd-Loss via npm-Shim (`tsc`, `vite` als npm-Scripts)                      | cwd-agnostische Flags: `-p src/tsconfig.json`, `-c src/vite.config.ts` |
| 4   | Vite `root: "src"` → Output landet in `src/dist/`                               | `outDir: "../dist"` explizit setzen                                    |
| 5   | `<script src="/src/main.tsx">` in `index.html` nicht resolvable                 | Relativ: `<script src="./main.tsx">`                                   |
| 6   | `cargo install tauri-cli` braucht ~5–10 Min Compile                             | `@tauri-apps/cli` via npm                                              |
| 7   | Binary-Filename = Cargo-Paketname (`my-copilot.exe`), NICHT `productName`       | Scripts müssen `my-copilot.exe` matchen                                |
| 8   | Tauri-Binary wird auch ohne `--bundles` gebaut (Artefakt liegt trotzdem da)     | `--no-bundle` Flag (separater Assemble-Step)                           |
| 9   | GitHub-Actions v4 läuft auf deprecated Node-20                                  | v5-Actions: `checkout@v5`, `setup-node@v5`, `upload-artifact@v5`       |
| 10  | Vite 8: `minify: "esbuild"` ist deprecated                                      | `minify: true` (Vite-8-Default = Oxc)                                  |
| 11  | `transformWithEsbuild` durch Plugins → `esbuild` muss als devDep                | `@vitejs/plugin-react@6` nutzt Oxc nativ, kein Workaround nötig        |

### CI-Status (laufende Pipeline)

| Tag-Phase       | Status    | DBID          | Notes                                                                                                                                                                                                                                                                                                           |
| --------------- | --------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **rc1–rc13**    | ❌ rot     | —             | diverse Fehler (siehe Lessons Learned)                                                                                                                                                                                                                                                                          |
| **rc14–rc17**   | ✅ grün    | `29704573326` | alle 14 Steps + Post-Steps success, ZIP-Bundle + GitHub-Release                                                                                                                                                                                                                                                 |
| **v0.1.0**      | ✅ grün    | `29724006762` | DBID `29724006762`, alle 14 Steps + 3 Post-Steps success (`07:14:46Z` → `07:23:34Z`, ~9 min), GitHub Release `v0.1.0` mit ZIP-Bundle, automatisch getaggt nach rc17                                                                                                                                             |
| **v0.1.0-rc18** | ⏳ pending | —             | ergibt sich logisch aus `c5873d7` (Rust-SDK-Migration) + 5 Folge-Fixes nach FF-Merge. Falls ohne neue Probleme grün, direkt zu `v0.1.1`                                                                                                                                                                         |
| **v0.1.0-rc19** | ⏳ pending | —             | **Streaming-Architektur v2** (M10, 2026-07-31): persistente Bridge, `chat_chunk`/`chat_done`/`chat_error`-Tauri-Events, `assistant.message_delta`-Streaming via `with_streaming(true)`, Session-ID-Trennung von Request-ID, `dedupe_v1_suffix`-Endpoint-Normalisierung. Erwartet: grün, dann direkt zu `v0.1.1` |

## Project Files

- `specs/SPEC-001-architecture-stack.md` — High-Level-Architektur und
  Tech-Entscheidungen
- `specs/SPEC-002-portable-bundle.md` — Folder-Layout, Pfad-Resolution,
  Distribution
- `specs/SPEC-003-byok-configuration.md` — config.json, DPAPI,
  Endpoint-Setup
- `specs/SPEC-004-bridge-tauri-rust.md` — Tauri-Rust Bridge (Copilot
  SDK Rust, Subprozess-Management, IPC-Methoden)
- `specs/SPEC-005-frontend-copilotkit-react.md` — Frontend mit
  React + CopilotKit UI-Schicht
- `specs/SPEC-006-config-dialog.md` — Konfigurations-Dialog (API Key,
  Base URL, System Prompt, MCP Servers)
- `LICENSE` — MIT-Lizenztext
- `DECISIONS.md` — Architektur- und Projekt-Entscheidungen mit Datum
  und Begründung (on-demand geladen)

## Workboard

**Board:** `my-copilot`
**Default-Workspace:** `C:\Users\Admin\.openclaw\workspace\projects\my-copilot` (Branch `main`)
**Worktree-Mode:** nein (direkt auf `main`)
**Eingerichtet am:** 2026-07-17

**Stats:** 9 Karten, 0 todo · 0 ready · 0 running · 0 blocked · 9 complete

### Karte-Status-Verlauf

- **2026-07-17 23:25** Karte #1 (Tauri-Skeleton) claimed → running
- **2026-07-17 23:35** Karte #1 verification: `cargo check` ✅ (9.44s)
- **2026-07-17 23:37** Karte #1 complete (Commit `f95dbf2`)
- **2026-07-17 23:39** Karte #2 (CLI-Bridge) claimed → running
- **2026-07-17 23:45** Karte #2 verification: `cargo check` ✅ (2.19s)
- **2026-07-17 23:47** Karte #2 complete (Commit `90052a5`)
- **2026-07-17 23:48** Karte #3 (IPC-API) claimed → running
- **2026-07-17 23:50** Karte #3 verification: `cargo check` ✅ (2.69s)
- **2026-07-17 23:51** Karte #3 complete (Commit `91f610d`)
- **2026-07-18 11:05** Karte #4 (Config-Management, Klartext) claimed → running
- **2026-07-18 11:09** Karte #4 verification: `cargo check` ✅ (0.77s)
- **2026-07-18 11:12** Karte #4 complete (Commit `369dd0e`)
- **2026-07-18 11:32** Karte #5 (BYOK-Config-Dialog UI) claimed → running
- **2026-07-18 11:35** Karte #5 verification: `tsc -p src/tsconfig.json` ✅
- **2026-07-18 11:37** Karte #5 complete (Commit `747a465`)
- **2026-07-18 11:40** Karte #6 (JSONL-Chat-History) claimed → running
- **2026-07-18 11:45** Karte #6 verification: `cargo check` ✅ (1.98s)
- **2026-07-18 11:46** Karte #6 complete (Commit `44aa9fc`)
- **2026-07-18 11:50** Karte #7 (Chat-UI mit CopilotKit) claimed → running
- **2026-07-18 11:55** Karte #7 verification: `tsc -p src/tsconfig.json` ✅
- **2026-07-18 11:56** Karte #7 complete (Commit `215e129`)
- **2026-07-18 12:00** Karte #8 (E2E-Smoke-Test) claimed → running
- **2026-07-18 12:05** Karte #8 verification: Test-Protokoll geschrieben (kein Cargo/TSC, Doku-only)
- **2026-07-18 12:06** Karte #8 complete (Commit `90d4e08`)
- **2026-07-18 12:10** Karte #9 (Build-Pipeline + Distribution) claimed → running
- **2026-07-18 12:15** Karte #9 verification: GitHub-Actions-Workflow geschrieben + getestet via YAML-Validierung
- **2026-07-18 12:16** Karte #9 complete (Commit pending)

### Karten-Liste

| #   | Karte                                                          | Priority | Status       | Labels                                   | Card-ID                                |
| --- | -------------------------------------------------------------- | -------- | ------------ | ---------------------------------------- | -------------------------------------- |
| 1   | Tauri-Skeleton aufsetzen (Cargo.toml, src-tauri/, Hello-World) | high     | **complete** | setup, tauri, milestone-1                | `a15846ee-201d-4a96-a2c7-48bcd47a700f` |
| 2   | Tauri-Rust CLI-Bridge (Subprozess + JSON-RPC via Stdin/Stdout) | high     | **complete** | bridge, rust, subprocess, milestone-2    | `26372b1f-1341-48fe-86d3-fad019be2305` |
| 3   | Tauri-IPC-API definieren (Commands + Events + Streaming)       | high     | todo         | ipc, tauri, milestone-2                  | `9fb7796f-dcc6-47d1-892b-98a9849e140f` |
| 4   | Config-Management (config.json, v1 Klartext + v3-DPAPI-TODO)   | high     | **complete** | config, rust, milestone-3                | `cff6cacd-cb5e-4700-981a-e915aef527a7` |
| 5   | BYOK-Config-Dialog UI (React + Tauri-IPC)                      | high     | **complete** | ui, config, react, milestone-3           | `fca83a1a-2c4b-48e6-a13f-6493d52d7c06` |
| 6   | JSONL-Chat-History (Sessions, Append-only, Read)               | normal   | **complete** | persistence, jsonl, rust, milestone-4    | `58b4d704-ce27-46a0-adf8-2b2dd7ad1cc7` |
| 7   | Chat-UI mit CopilotKit (Streaming + Tool-Calls)                | normal   | **complete** | ui, chat, copilotkit, react, milestone-4 | `ea56fa8a-d65d-4768-ae2c-ef31d3a7cf94` |
| 8   | End-to-End-Smoke-Test (manuelles Test-Protokoll)               | high     | **complete** | test, e2e, milestone-5                   | `291b9b51-2106-44a7-ae40-189079bf7bd1` |
| 9   | Build-Pipeline + Distribution (ZIP + GitHub Release)           | low      | **complete** | build, distribution, milestone-6         | `54e45cbf-a3ed-4916-bcf8-49017f8dd7e6` |

### Milestone-Übersicht

- **M1** Tauri-Skeleton → Karte #1
- **M2** Bridge + IPC → Karten #2, #3
- **M3** Config + UI → Karten #4, #5
- **M4** Chat → Karten #6, #7
- **M5** Test → Karte #8
- **M6** Build + Distribution → Karte #9

### Lifecycle-Workflow

Karten liegen in `todo`. Wenn wir anfangen zu arbeiten:
1. `workboard_specify` — Karte klären (Acceptance-Kriterien, ggf. Decompose)
2. `workboard_dispatch` — Karte auf `ready` setzen
3. `workboard_claim` — Claim-Token holen (sperrt die Karte für diesen Agent)
4. `workboard_heartbeat` — während der Arbeit (verhindert stale)
5. `workboard_proof` — Test-Ergebnisse / Screenshots anhängen
6. `workboard_complete` — Summary + Artifact-Links

## Offene Punkte

**Aus SPEC-006 abgeleitet:**

- System-Prompt-Default: leer vs. mitgelieferter Default?
- MCP/SystemPrompt-Übergabe-Mechanik an aktuelle Copilot-CLI anpassen
- MCP-Server-Bundling: Standard-Server mitliefern oder nur User-Config?
- Hot-Reload vs. Restart für System Prompt / MCP Servers *(v3-TODO)*
- Per-Session-Override für System Prompt / Modell
- MCP-Server-Templates in der UI

**v3-Sicherheits-/Distribution-Features** *(Martins Direktive 2026-07-18 11:41:20)*:
- **Code-Signing** — EV-Code-Signing-Zertifikat kaufen + `signtool.exe` in CI-Pipeline — siehe DECISIONS.md
- **Auto-Update-Mechanismus** — GitHub-Releases-Checker oder Squirrel/Sparkle-Wrapper — siehe DECISIONS.md
- **DPAPI-Verschlüsselung** — `keyring`-Crate (Windows Credential Manager → DPAPI, macOS-Keychain, Linux-Secret-Service) — siehe DECISIONS.md

**Spec-Follow-ups (Aufräumarbeiten):**

- SPEC-003 § config.json Schema um `systemPrompt` + `mcpServers` erweitern
- SPEC-004 § IPC-Methoden Tabelle auf erweiterte Payload-Shape anpassen
- SPEC-002 § Folder-Layout: `chat-history.db` → `sessions/{session-id}.jsonl`