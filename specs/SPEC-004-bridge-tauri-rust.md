# SPEC-004 — Tauri-Rust Bridge (Copilot SDK Rust)

**Status:** Implementierung in Arbeit
**Datum:** 2026-07-17 (initial) / 2026-07-17 (rewrite: C# → Rust)
**Bezug:** SPEC-001 § Tech-Entscheidungen (Tauri-Rust Bridge) ·
SPEC-005 § IPC-Anbindung · `DECISIONS.md` § Architektur-Verschlankung

## Übersicht

Tauri-Rust ist nicht nur die App-Shell, sondern auch die **einzige
Bridge zwischen Frontend und GitHub Copilot SDK for Rust**. Die Bridge
verwendet den offiziellen SDK-Client statt manueller ACP-Nachrichten,
konfiguriert BYOK über `ProviderConfig` und ruft Session-/Message-
Operationen über die SDK-API auf.

**Wichtig:** Es wird **kein Port** für IPC geöffnet — weder HTTP noch
Named Pipe noch TCP. Alle Inter-Prozess-Kommunikation läuft über
OS-Pipes (Stdin/Stdout des Subprozesses).

## Komponenten-Stapel

```
React (lokale Chat-UI)
  ↕ Tauri-IPC (Commands + Events, intern — kein Netzwerk)
Tauri-Rust (App-Shell + Bridge)
    ├── github_copilot_sdk::Client
    └── SessionConfig + ProviderConfig (BYOK)
Copilot CLI (Node.js-App, embedded)
  ↕ HTTPS / SSE
OpenAI-kompatibler Endpoint
```

**Zwei Prozesse zur Laufzeit**: Tauri-Rust (App-Shell + Bridge) ·
Node.js+CLI.

## SDK-Architektur

```
React UI                    Tauri-Rust Bridge              github_copilot_sdk
    │                              │                                 │
    │ invoke('chat_send')          │                                 │
    ├─────────────────────────────►│                                 │
    │                              │ Client::start(...)              │
    │                              ├────────────────────────────────►│
    │                              │ create_session(config)         │
    │                              ├────────────────────────────────►│
    │                              │ session.send_and_wait(...)     │
    │                              ├────────────────────────────────►│
    │                              │ ◄──── assistant response ──────│
    │ ◄────── final response ──────│                                 │
```

**Zwei IPC-Layer**, beide ohne Netzwerk:

1. **React ↔ Tauri-Rust**: Tauri-IPC über interne Message-Pipes
   (Tauri-eigenes Protokoll, kein TCP).
2. **Tauri-Rust ↔ CLI**: Stdin/Stdout-Pipes des Subprozesses
   (JSON-RPC, **kein Port**, kein HTTP, kein Named Pipe).

## SDK-Client-Lifecycle

Die Bridge verwendet den offiziellen Rust-SDK-Client. Dieser verwaltet
die Copilot-CLI-Runtime selbst und kapselt Spawn, Health-Checks,
Transport und Session-Erzeugung.

```rust
use std::sync::Arc;
use github_copilot_sdk::{Client, ClientOptions, CliProgram, MessageOptions};
use github_copilot_sdk::handler::ApproveAllHandler;
use github_copilot_sdk::types::{ProviderConfig, SessionConfig};

let mut provider = ProviderConfig::default();
provider.provider_type = Some("openai".to_string());
provider.base_url = "https://api.minimax.io/v1".to_string();
provider.api_key = Some("...".to_string());
provider.wire_api = Some("completions".to_string());

let client = Client::start(
    ClientOptions::default()
        .with_program(CliProgram::Path(copilot_binary_path))
).await?;

let session = client.create_session(
    SessionConfig::default()
        .with_permission_handler(Arc::new(ApproveAllHandler))
        .with_provider(provider)
).await?;

let response = session
    .send_and_wait(MessageOptions::new("Hello"))
    .await?;
```

**Wichtig:** Die Bridge verwaltet weiterhin den Pfad zur eingebetteten
CLI-Binary, aber nicht mehr das ACP- oder JSON-RPC-Protokoll selbst.

## Bridge-Verantwortung

Die Rust-Bridge ist für folgende Aufgaben zuständig:

1. Laden und Persistieren der lokalen BYOK-Konfiguration.
2. Auflösen oder Bundlen der Copilot-CLI-Runtime.
3. Übersetzen der lokalen Config in `ProviderConfig` und `SessionConfig`.
4. Aufruf von `Client::start`, `create_session`, `send` bzw. `send_and_wait`.
5. Rückgabe der Assistant-Response an das Frontend via Tauri-Commands.

## IPC-Methoden (Tauri-Commands + Events)

| Methode           | Richtung        | Payload                                               |
| ----------------- | --------------- | ----------------------------------------------------- |
| `chat.send`       | Frontend → Rust | `{message: string}`                                   |
| `chat.cancel`     | Frontend → Rust | `{request_id: string}`                                |
| `config.get`      | Frontend → Rust | `{}` → `{endpoint, model, systemPrompt, mcpServers}`  |
| `config.set`      | Frontend → Rust | `{endpoint, apiKey, model, systemPrompt, mcpServers}` |
| `config.test`     | Frontend → Rust | `{endpoint, apiKey}` → `{ok, models}`                 |
| `process.health`  | Frontend → Rust | `{}` → `{cli_running, cli_ready}`                     |
| `process.restart` | Frontend → Rust | `{}` (Subprozess neu starten)                         |

V1 kann weiter non-streaming bleiben; später kann `Session::subscribe()`
für echtes Event-/Chunk-Streaming genutzt werden.

## Persistenz

- **Chat-History**: JSONL, eine Datei pro Session unter
  `./data/sessions/{session-id}.jsonl`
  - Schema pro Zeile: `{id, request_id, role, content, ts, model, tokens}`
  - Append-only (robust gegen Teil-Schreibfehler, einfache Implementierung)
  - Human-readable (Notepad/VSCode reicht für Inspection)
  - Kein Native-Dep (kein `Microsoft.Data.Sqlite`)
  - Per-Session-Files = einzelne Sessions einfach löschbar/restorbar
- **Logs**: `./data/logs/app-YYYY-MM-DD.log` (rolling, max 10 MB / File)
- **Cache**: `./data/cache/` für Tool-Call-Results, Embeddings etc.

**Trade-off:** Kein effizientes Querying (Full-Read für Stats/Filter).
Für v1 mit ~100–1000 Sessions OK. Falls später nötig: Sidecar-Index-
File oder Migration zu SQLite.

## Fehlerbehandlung

| Fehler                      | Reaktion                                   |
| --------------------------- | ------------------------------------------ |
| CLI-Runtime nicht gefunden  | Setup-Screen mit Hinweis                   |
| SDK-Client Startfehler      | User-Notification + Log-Ausgabe            |
| BYOK-Provider 401           | „API-Key ungültig" → Settings-Dialog       |
| BYOK-Endpoint 429           | Exponential Backoff + Fallback-Modell      |
| BYOK-Endpoint Network-Error | Retry mit User-Bestätigung                 |
| SDK-RPC-Fehler              | User-Notification „Bridge antwortet nicht" |

## Offene Punkte

- **SDK-API-Stabilität**: Rust-SDK ist jung, aber offiziell. Mitigation:
    Integrationslogik bleibt in einer Bridge-Schicht gekapselt.
- **Streaming-Granularität**: `Session::subscribe()` vs. `send_and_wait()`.
    V1 nutzt Einfachheit; Streaming kann später ergänzt werden.
- **Bundling-Strategie**: SDK-`bundled-cli` vs. eigene Sidecar-Binary.
    Für das bestehende Portable-Bundle muss die endgültige Strategie im
    Build/Release-Pfad konsolidiert werden.
- **Schema-Migration JSONL**: bei v1+ Schema-Changes für JSONL
  (rückwärtskompatibel via Default-Werte pro Feld).

## Quellen

- `github/copilot-sdk` (Rust-Variante) — offizielles SDK
- Tauri 2 Docs — Commands + Events — tauri.app
- `tokio::process::Command` — async Subprozess-Management
- JSONL-Pattern in Rust — `serde_json` + `tokio::fs::File`
- Tauri 2 Docs — Sidecar-Pattern (`externalBin`)