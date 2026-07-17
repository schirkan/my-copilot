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
│   ↕ IPC (Tauri-Commands / Events)                          │
│ Tauri Rust Core (Sidecar-Bridge)                           │
│   ↓ Process.Start (Sidecar-Pattern)                        │
│ C# Backend (.NET 9, self-contained AOT)                    │
│   ↓ Copilot SDK .NET → JSON-RPC über Stdin/Stdout          │
│ Node.js (embedded, portable)                               │
│   ↓ startet                                                 │
│ GitHub Copilot CLI (Node.js-App)                           │
│   ↓ HTTPS / SSE                                             │
│ OpenAI-kompatibler Endpoint                                │
│   (Azure OpenAI · self-hosted vLLM/LM-Studio · OpenRouter) │
└────────────────────────────────────────────────────────────┘
```

**Drei Prozesse zur Laufzeit**: Tauri (Rust) · C# Backend ·
Node.js+CLI.

## Tech-Entscheidungen

### Tauri 2 (App-Shell)

- **Warum**: Native Windows-Binaries (Rust), kein Electron-Overhead.
  WebView2 als Renderer (Edge-Engine, auf Win 11 vorinstalliert).
- **Sidecar-Pattern**: Externe Prozesse als „Sidecars" managed — passt
  zu C# + Node.js-Setup.
- **Trade-off**: Rust-Backend klein, IPC-Overhead zu C# minimal.

### C# Backend (.NET 9)

- **Warum**: Martins 20+ Jahre .NET-Expertise.
- **Copilot SDK**: Offizielles .NET-Paket verfügbar
  (`github/copilot-sdk` Repo, Multi-Sprache: TS / Python / Go / .NET /
  Java / Rust).
- **Self-contained AOT**: Kompiliertes Binary ohne .NET-Framework-
  Installation beim User.
- **Trade-off**: Type-Safety + Performance + Martin-Vertrautheit.

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
- **Runtime**: Im App-Bundle mitgeliefert, vom C#-Backend absolut
  gepfadet gestartet. NODE_PATH nicht ändern, sondern explizit an
  Prozess-ENV übergeben.

## Trade-offs (ehrlich)

| Vorteil                                | Nachteil                                  |
|----------------------------------------|-------------------------------------------|
| Copilot-Tools out-of-the-box           | +100 MB Bundle-Size (Node.js+CLI+Deps)    |
| BYOK native                            | 3 Prozesse zur Laufzeit (Tauri/C#/Node)   |
| Portable Folder, kein Installer        | Build-Pipeline komplexer                  |
| C# Backend passt zu Martins Skills     | TS-Ökosystem-Hilfen etwas entfernt       |
| Microsoft-nativ (Copilot SDK)          | Breaking-Changes bei SDK-Updates möglich  |

## Plattform-Annahmen (fix)

- **Zielplattform**: Windows 11
- **WebView2 als gegeben vorausgesetzt** (Edge-Component, kein
  Bootstrapper)
- **BYOK zwingend** (kein GitHub-Copilot-Abo nötig)
- **Portable Folder** (kopierbar, kein MSI / NSIS)
- **Build-Umgebung (Dev/CI)**: Node.js v22+, npm v10+, Rust toolchain,
  .NET SDK 9+, Tauri CLI

## Offene Punkte (Architektur)

- **Sidecar-Pattern-Detail**: Tauri spawnt C# direkt, oder C# als
  externer Daemon via Service? — siehe SPEC-004
- **IPC-Protokoll**: Tauri-Rust ↔ C# über HTTP (localhost) oder Named
  Pipe?
- **React-Bundle**: Vite-Build mit Node.js zur Build-Zeit akzeptiert
  (Node.js nur im Dev/CI, nicht im Output)

## Quellen

- VSCode 1.129 Release Notes (15.07.2026) — VSCode Agent Host Protocol
- `github/copilot-sdk` Repository
- CopilotKit Documentation (copilotkit.ai)
- Tauri 2 Documentation (tauri.app)
- nodejs.org — Portable Node.js v22+ Distributions