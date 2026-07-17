# SPEC-001 — Architecture Stack (My Copilot)

**Status:** Planungs-Phase, kein Code
**Datum:** 2026-07-17
**Auslöser:** Diskussion 16.–17.07.2026 im ai-ideas-Topic (Copilot SDK,
BYOK, Agent-Runtime, portable App).

## Übersicht

`My Copilot` ist eine portable Desktop-App für KI-Agent-Workflows.
Architektur folgt dem „embedded Node.js"-Ansatz (vorher evaluiert als
„Option ②" der Diskussion), um die GitHub Copilot SDK-UX
(Read/Write/Edit/Bash-Tooling out-of-the-box) zu nutzen, ohne dass der
End-User Node.js separat installieren muss.

## Komponenten-Stapel

```
┌────────────────────────────────────────────────────────────┐
│ Tauri WebView (CopilotKit React Frontend)                   │
│   ↕ Tauri-IPC (Commands + Events, intern — kein Netzwerk)  │
│ Tauri Rust Core (App-Shell + Bridge)                       │
│   ├── spawnt Subprozess                                    │
│   └── JSON-RPC via Stdin/Stdout-Pipes (Copilot SDK Rust)   │
│ GitHub Copilot CLI (Node.js-App, embedded)                 │
│   ↓ HTTPS / SSE                                             │
│ OpenAI-kompatibler Endpoint                                 │
│   (Azure OpenAI · self-hosted vLLM/LM-Studio · OpenRouter)  │
└────────────────────────────────────────────────────────────┘
```

**Zwei Prozesse zur Laufzeit**: Tauri-Rust (App-Shell + Bridge) ·
Node.js+CLI.

**Wichtig**: Es wird **kein Port** für IPC geöffnet — weder HTTP
noch Named Pipe noch TCP. Alle Inter-Prozess-Kommunikation läuft
über OS-Pipes (Stdin/Stdout des Subprozesses).

## Tech-Entscheidungen

### Tauri 2 (App-Shell)

- **Warum**: Native Windows-Binaries (Rust), kein Electron-Overhead.
  WebView2 als Renderer (Edge-Engine, auf Win 11 vorinstalliert).
- **Sidecar-Pattern**: Externe Prozesse als „Sidecars" managed — passt
  zu C# + Node.js-Setup.
- **Trade-off**: Rust-Backend klein, IPC-Overhead zu C# minimal.

### Tauri-Rust Bridge

- **Warum**: Eine Schicht weniger (Tauri-Rust übernimmt Bridge-Logik,
  kein separates Backend nötig), kein Port für IPC nötig
  (Stdin/Stdout-Pipes reichen).
- **Copilot SDK Rust**: Offizielles Rust-Paket verfügbar
  (`github/copilot-sdk` Repo, Multi-Sprache: TS / Python / Go /
  .NET / Java / Rust).
- **IPC**: Stdin/Stdout-Pipes via `tokio::process::Command` —
  **kein HTTP, kein Named Pipe, kein TCP-Port** (siehe
  `DECISIONS.md` § Architektur-Verschlankung).
- **Trade-off**: Rust-Lernkurve (Martin 20+ Jahre .NET).
  Mitigation: Tauri ist Rust-nativ, große Community, viele
  Beispiele für genau dieses Subprozess-Pattern.

### GitHub Copilot CLI (statt direkter OpenAI-Aufruf)

- **Warum**: Copilot CLI bietet **Read/Write/Edit/Bash/Glob/Grep/
  WebFetch** out-of-the-box. Diese Tools sind schwer nachzubauen und
  sparen Monate an Eigenentwicklung.
- **Stand**: v1.0.6 vom 08.07.2026, aktiv gepflegt von GitHub.
- **BYOK-Engine**: identisch zu VSCode-1.129-Copilot-Agent-Host.
- **Trade-off**: +100 MB Bundle-Size, dritter Prozess.

### CopilotKit React (Frontend)

- **Warum**: Speziell für Copilot-SDK-Backend gebaut, Generative UI
  out-of-the-box.
- **API**: `useCopilotChat`, `<CopilotKit>`, `<CopilotPopup>` — React-
  native Hooks / Komponenten.
- **Alternative evaluiert**: `assistant-ui` (YC W25, framework-
  agnostic). Habe CopilotKit gewählt wegen direktem Bezug zu
  Copilot SDK.

### Embedded Node.js

- **Warum**: Copilot CLI ist eine Node.js-Anwendung. Wir wollen den User
  nicht zwingen, Node.js separat zu installieren.
- **Bezugsquelle**: Portable Node.js v22+ von nodejs.org (~30 MB).
- **Runtime**: Im App-Bundle mitgeliefert, von der Tauri-Rust
  Bridge absolut gepfadet gestartet. NODE_PATH nicht ändern,
  sondern explizit an Prozess-ENV übergeben.

## Trade-offs (ehrlich)

| Vorteil                                | Nachteil                                  |
|----------------------------------------|-------------------------------------------|
| Copilot-Tools out-of-the-box           | +100 MB Bundle-Size (Node.js+CLI+Deps)    |
| BYOK native                            | 2 Prozesse zur Laufzeit (Tauri/CLI)       |
| Portable Folder, kein Installer        | Rust-Lernkurve (Martin: 20+ Jahre .NET)   |
| Tauri-Rust als Bridge = eine Sprache   | Breaking-Changes bei SDK-Updates möglich  |
| Kein C# / kein Port / kein HTTP-IPC    | TS-Ökosystem-Examples entfernt            |

## Plattform-Annahmen (fix)

- **Zielplattform**: Windows 11
- **WebView2 als gegeben vorausgesetzt** (Edge-Component, kein
  Bootstrapper)
- **BYOK zwingend** (kein GitHub-Copilot-Abo nötig)
- **Portable Folder** (kopierbar, kein MSI / NSIS)
- **Build-Umgebung (Dev/CI)**: Node.js v22+, npm v10+, Rust toolchain,
  Tauri CLI

## Offene Punkte (Architektur)

- **Sidecar-Lifecycle**: Tauri 2 `externalBin` (statisch konfiguriert,
  automatischer Lifecycle) vs. manuelle `tokio::process::Command`-
  Verwaltung (dynamischer, eigener Restart-Loop) — siehe SPEC-004 §
  Offene Punkte.
- **React-Bundle**: Vite-Build mit Node.js zur Build-Zeit akzeptiert
  (Node.js nur im Dev/CI, nicht im Output).

## Quellen

- VSCode 1.129 Release Notes (15.07.2026) — VSCode Agent Host Protocol
- `github/copilot-sdk` Repository
- CopilotKit Documentation (copilotkit.ai)
- Tauri 2 Documentation (tauri.app)
- nodejs.org — Portable Node.js v22+ Distributions