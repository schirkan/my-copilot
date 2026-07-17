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
- **Kein Code geschrieben** — Specs dokumentieren ausschließlich die
  geplante Architektur.
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

_Noch nicht eingerichtet._ Wird angelegt, sobald die Implementierung
startet (Workboard-Pflicht ab ≥3 Sub-Schritten pro
`projects/PROJECT-RULES.md`).

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