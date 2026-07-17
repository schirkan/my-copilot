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

| Komponente    | Technologie              | Sprache           |
|---------------|--------------------------|-------------------|
| App-Shell + Bridge | Tauri 2 + Copilot SDK Rust | Rust         |
| Subprozess    | Node.js v22+ + Copilot CLI | JavaScript     |
| Frontend      | CopilotKit React         | TypeScript / JSX  |
| LLM-Provider  | OpenAI-kompatibel        | n/a               |

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
- **Kein Code geschrieben** *(vor 2026-07-17 M1)* — Specs dokumentierten
  ausschließlich die geplante Architektur.
- **Kein Workboard-Board** — wird angelegt, sobald Implementierung startet
  (Workboard-Pflicht ab ≥3 Sub-Schritten, siehe
  `projects/PROJECT-RULES.md`).

## Git

| Feld                    | Wert                                                              |
|-------------------------|-------------------------------------------------------------------|
| **Repo-Typ**            | GitHub (public)                                                   |
| **Pfad / URL**          | `https://github.com/schirkan/my-copilot`                          |
| **Lokaler Pfad**        | `C:\Users\Admin\.openclaw\workspace\projects\my-copilot`          |
| **Remote(s)**           | `origin` → `https://github.com/schirkan/my-copilot.git`           |
| **Default-Branch**      | `main`                                                            |
| **Eingerichtet am**     | 2026-07-17                                                        |
| **`.gitignore`-Status** | vorhanden                                                         |
| **Lizenz**              | MIT (siehe `LICENSE`)                                            |

> Hinweis: Der OpenClaw-Workspace-Root (`C:\Users\Admin\.openclaw\workspace`)
> ist ein separates Git-Repo. `projects/my-copilot/` ist dort **nicht**
> getrackt — eigenständiges Repo.

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
  CopilotKit React
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

**Stats:** 9 Karten, 0 todo · 0 ready · 0 running · 0 blocked · 2 complete

### Karte-Status-Verlauf

- **2026-07-17 23:25** Karte #1 (Tauri-Skeleton) claimed → running
- **2026-07-17 23:35** Karte #1 verification: `cargo check` ✅ (9.44s)
- **2026-07-17 23:37** Karte #1 complete (Commit `f95dbf2`)
- **2026-07-17 23:39** Karte #2 (CLI-Bridge) claimed → running
- **2026-07-17 23:45** Karte #2 verification: `cargo check` ✅ (2.19s)
- **2026-07-17 23:46** Karte #2 complete (Commit pending)

### Karten-Liste

| # | Karte | Priority | Status | Labels | Card-ID |
|---|---|---|---|---|---|
| 1 | Tauri-Skeleton aufsetzen (Cargo.toml, src-tauri/, Hello-World) | high | **complete** | setup, tauri, milestone-1 | `a15846ee-201d-4a96-a2c7-48bcd47a700f` |
| 2 | Tauri-Rust CLI-Bridge (Subprozess + JSON-RPC via Stdin/Stdout) | high | **complete** | bridge, rust, subprocess, milestone-2 | `26372b1f-1341-48fe-86d3-fad019be2305` |
| 3 | Tauri-IPC-API definieren (Commands + Events + Streaming) | high | todo | ipc, tauri, milestone-2 | `9fb7796f-dcc6-47d1-892b-98a9849e140f` |
| 4 | Config-Management (config.json + DPAPI für apiKey) | high | todo | config, rust, milestone-3 | `cff6cacd-cb5e-4700-981a-e915aef527a7` |
| 5 | BYOK-Config-Dialog UI (React + Tauri-IPC) | high | todo | ui, config, react, milestone-3 | `fca83a1a-2c4b-48e6-a13f-6493d52d7c06` |
| 6 | JSONL-Chat-History (Sessions, Append-only, Read) | normal | todo | persistence, jsonl, rust, milestone-4 | `58b4d704-ce27-46a0-adf8-2b2dd7ad1cc7` |
| 7 | Chat-UI mit CopilotKit (Streaming + Tool-Calls) | normal | todo | ui, chat, copilotkit, react, milestone-4 | `ea56fa8a-d65d-4768-ae2c-ef31d3a7cf94` |
| 8 | End-to-End-Smoke-Test (manuelles Test-Protokoll) | high | todo | test, e2e, milestone-5 | `291b9b51-2106-44a7-ae40-189079bf7bd1` |
| 9 | Build-Pipeline + Distribution (ZIP + GitHub Release) | low | todo | build, distribution, milestone-6 | `54e45cbf-a3ed-4916-bcf8-49017f8dd7e6` |

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
- Hot-Reload vs. Restart für System Prompt / MCP Servers
- Per-Session-Override für System Prompt / Modell
- MCP-Server-Templates in der UI

**Spec-Follow-ups (Aufräumarbeiten):**

- SPEC-003 § config.json Schema um `systemPrompt` + `mcpServers` erweitern
- SPEC-004 § IPC-Methoden Tabelle auf erweiterte Payload-Shape anpassen
- SPEC-002 § Folder-Layout: `chat-history.db` → `sessions/{session-id}.jsonl`