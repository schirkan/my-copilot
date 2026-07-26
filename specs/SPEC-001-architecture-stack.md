# SPEC-001 — Architecture Stack (My Copilot)

**Status:** Planungs-Phase, kein Code
**Datum:** 2026-07-17
**Auslöser:** Diskussion 16.–17.07.2026 im ai-ideas-Topic (Copilot SDK,
BYOK, Agent-Runtime, portable App).

## Übersicht

`My Copilot` ist eine portable Desktop-App für KI-Agent-Workflows.
Architektur trennt klar zwischen **UI-Schicht** und **Runtime-
Schicht**: Das Frontend kann CopilotKit-Komponenten für die Chat-UX
verwenden, während Tauri-Rust das offizielle Rust-SDK als einzigen
Runtime-/Agent-Layer nutzt.

## Komponenten-Stapel

```
┌────────────────────────────────────────────────────────────┐
│ Tauri WebView (React + CopilotKit UI)                      │
│   ↕ Tauri-IPC (Commands + Events, intern — kein Netzwerk)  │
│ Tauri Rust Core (App-Shell + Bridge)                       │
│   ├── github_copilot_sdk::Client                           │
│   └── ProviderConfig / SessionConfig                       │
│ GitHub Copilot CLI runtime                                 │
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

### GitHub Copilot SDK + CLI Runtime

- **Warum**: Das offizielle SDK kapselt Session-Lifecycle,
  Permission-Handling, Hooks und BYOK-Konfiguration auf einem höheren
  Level als ein manueller ACP-Client.
- **Runtime darunter**: Die Copilot-CLI-Engine bleibt erhalten, wird
  aber über den SDK-Client angesprochen statt per handgebautem ACP.
- **Trade-off**: Die Runtime bleibt ein zusätzlicher Prozess bzw. eine
  eingebettete Laufzeitkomponente.

### React-Frontend + CopilotKit UI

- **Warum**: CopilotKit kann für Chat-Präsentation, Message-Komponenten
  und perspektivisch Tool-/Generative-UI nützlich bleiben.
- **Wichtig**: CopilotKit wird **nicht** als Runtime- oder Transport-
  Layer genutzt. Kein `runtimeUrl`, kein `publicApiKey`, kein direkter
  Cloud-Handshake aus dem Frontend.
- **Datenfluss**: Submit/Antworten laufen ausschließlich über
  Tauri-IPC ↔ Rust-SDK.

### Embedded Node.js

- **Warum**: Copilot CLI ist eine Node.js-Anwendung. Wir wollen den User
  nicht zwingen, Node.js separat zu installieren.
- **Bezugsquelle**: Portable Node.js v22+ von nodejs.org (~30 MB).
- **Runtime**: Im App-Bundle mitgeliefert, von der Tauri-Rust
  Bridge absolut gepfadet gestartet. NODE_PATH nicht ändern,
  sondern explizit an Prozess-ENV übergeben.

## Trade-offs (ehrlich)

| Vorteil                                  | Nachteil                                                   |
| ---------------------------------------- | ---------------------------------------------------------- |
| Offizieller SDK-Layer statt ACP-Bastelei | Zusätzliche Runtime-Komponente bleibt                      |
| BYOK offiziell dokumentiert              | Rust-SDK jung, API kann sich bewegen                       |
| Portable Folder, kein Installer          | Rust-Lernkurve (Martin: 20+ Jahre .NET)                    |
| Tauri-Rust als Bridge = eine Sprache     | Bundle-/Runtime-Strategie muss sauber sein                 |
| Kein C# / kein Port / kein HTTP-IPC      | Saubere Trennung UI vs. Runtime muss diszipliniert bleiben |

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