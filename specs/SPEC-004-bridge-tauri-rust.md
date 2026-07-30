# SPEC-004 — Tauri-Rust Bridge (Copilot SDK Rust)

**Status:** Implementiert (`github-copilot-sdk = "1"`, `bundled-cli` Default) + Streaming
**Datum:** 2026-07-17 (initial) / 2026-07-17 (rewrite: C# → Rust) /
2026-07-26 (Runtime-Migration: manueller ACP-Handshake → SDK-Client) /
2026-07-30 (Update für SDK 1.0.8, `bundled-cli`-Default, Endpoint-Normalisierung) /
2026-07-30 (Streaming-Implementierung: `Session::subscribe()`, persistente Bridge)
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
    │   └── bundled-cli Feature: CLI als komprimiertes Archive im
    │       Binary, wird zur Laufzeit vom SDK aus
    │       target/<profile>/copilot.exe geladen (Node.js SEA /
    │       Single Executable Application -- kein separates Node.js
    │       noetig)
    └── SessionConfig + ProviderConfig (BYOK)
OpenAI-kompatibler Endpoint
```

**Ein Prozess zur Laufzeit** (Tauri-Rust). Die CLI wird vom SDK intern
als Subprozess gespawnt (`tokio::process::Command` mit Stdin/Stdout-
Pipes) -- kein separater Node.js-Prozess von uns mehr (war bis
`c5873d7` obsolet). Das vorherige Setup mit embedded Node.js + separatem
`copilot-cli/`-Ordner ist Geschichte; siehe SPEC-002 (Obsolet seit
`c5873d7`).

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

## SDK-Client-Lifecycle (SDK 1.0.8, `bundled-cli` Default)

Die Bridge verwendet den offiziellen Rust-SDK-Client. Das `bundled-cli`-
Feature (Default seit SDK 1.0) laedt die CLI selbst aus den
offiziellen GitHub-Releases herunter, verifiziert die SHA256 und
entpackt sie nach `target/<profile>/copilot.exe`. Die Bridge muss sich
**nicht** mehr um Pfad-Auflösung, Download oder Sidecar-Bundling
kümmern.

```rust
use std::sync::Arc;
use github_copilot_sdk::{Client, ClientOptions, MessageOptions};
use github_copilot_sdk::handler::ApproveAllHandler;
use github_copilot_sdk::types::{ProviderConfig, SessionConfig};

// Kein expliziter Program-Pfad noetig -- Client::start nimmt die vom
// SDK gebundelte Binary (siehe Cargo.toml: bundled-cli = Default).
let client = Client::start(ClientOptions::default()).await?;

let session = client.create_session(
    SessionConfig::default()
        .with_permission_handler(Arc::new(ApproveAllHandler))   // v1.x Naming!
        .with_model(byok_config.model.clone())
        .with_provider(build_provider_config(&byok_config))
).await?;

let response = session
    .send_and_wait(
        MessageOptions::new(message).with_wait_timeout(Duration::from_secs(60))
    )
    .await?;
```

**Wichtig (Breaking Changes seit SDK 0.1):**

- `SessionConfig::with_handler(...)` heißt in SDK 1.x
  **`with_permission_handler(...)`** (war ein Tippfehler in der alten
  Spec, hier explizit richtiggestellt).
- `CliProgram::Path(...)` ist nicht mehr nötig; `ClientOptions::default()`
  reicht. Damit fällt auch der gesamte Sidecar-Pfad-Auflösungs-Code weg
  (siehe [`src-tauri/src/copilot/process.rs`](../src-tauri/src/copilot/process.rs)
  -- `resolve_copilot_binary_path()` wurde in Commit `a1aa5bf` entfernt).
- Die Bridge verwaltet **keine** CLI-Pfade mehr -- das SDK macht alles.

### Endpoint-Normalisierung (BYOK)

OpenAI-kompatible Provider erwarten `base_url` **ohne** `/v1`-Suffix
(wird vom SDK für `wire_api = "completions"` automatisch angehängt).
User können die URL aber **mit oder ohne** `/v1` eingeben -- die Bridge
normalisiert in [`build_provider_config()`](../src-tauri/src/copilot/bridge.rs)
via `strip_v1_suffix()`:

```rust
fn strip_v1_suffix(endpoint: &str) -> String {
    let trimmed = endpoint.trim_end_matches('/');
    match trimmed.strip_suffix("/v1") {
        Some(base) => base.to_string(),
        None => trimmed.to_string(),
    }
}
```

Dieselbe Normalisierung gilt für `config_test` (siehe
[`config.rs`](../src-tauri/src/commands/config.rs)) -- sonst liefert
ein Test mit `https://api.openai.com/v1` ein 404 auf
`https://api.openai.com/v1/v1/models`.

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
| `chat_send`       | Frontend → Rust | `{message: string}` → `request_id` (sofort)           |
| `chat_cancel`     | Frontend → Rust | `{requestId: string}` → `session.abort()`             |
| `chat.chunk`      | Rust → Frontend | `{request_id, delta, accumulated}` (Streaming-Event)  |
| `chat.done`       | Rust → Frontend | `{request_id, content}` (Session fertig)              |
| `chat.error`      | Rust → Frontend | `{request_id, error}` (Session-Fehler)                |
| `config_get`      | Frontend → Rust | `{}` → `{endpoint, model, systemPrompt, mcpServers}`  |
| `config_set`      | Frontend → Rust | `{endpoint, apiKey, model, systemPrompt, mcpServers}` |
| `config_test`     | Frontend → Rust | `{endpoint, apiKey}` → `{ok, models}`                 |
| `process_health`  | Frontend → Rust | `{}` → `{cli_running, cli_ready}`                     |
| `process_restart` | Frontend → Rust | `{}` (Subprozess neu starten)                         |

Siehe [`src-tauri/src/commands/chat.rs`](../src-tauri/src/commands/chat.rs)
für die volle Streaming-Implementierung.

## Streaming-Architektur (v2)

`chat_send` returnt **sofort** die `request_id` (statt der fertigen
Antwort) und emittiert die Assistant-Response als Tauri-Events.
Implementiert in [`src-tauri/src/commands/chat.rs`](../src-tauri/src/commands/chat.rs).

### Lifecycle pro Chat-Message

```
Frontend                         Tauri-Rust                          SDK
  │                                  │                                │
  │ invoke('chat_send', message, session_id?)│                        │
  ├─────────────────────────────────►│                                │
  │                                  │ persist user message (JSONL)   │
  │                                  │ ensure_bridge() (lazy client)  │
  │                                  │ create_session() [with_streaming(true)] │
  │ ◄────── {session_id, request_id} ──│                                │
  │                                  │ subscribe() (broadcast)        │
  │                                  │ session.send(message)          │
  │                                  │ spawn stream-loop task         │
  │                                  │                                │
  │                                  │ ◄── assistant.message_delta ───┤ (streaming)
  │ ◄── emit('chat_chunk') ─────────│  (delta, accumulated)          │
  │     (x N)                        │                                │
  │                                  │ ◄── session.idle (oder assistant.idle) ┤
  │ ◄── emit('chat_done') ──────────│                                │
  │                                  │ persist assistant message      │
  │                                  │ session.disconnect()           │
  │                                  │ remove from active_session     │
```

### Persistente Bridge

Vorher (v1 non-streaming): pro `chat_send` wurde ein neuer
`Client::start()` Roundtrip gemacht, dann `client.stop()` im Drop.
Bei schnellem Chat (~1 Message/Sekunde) ist das spuerbar Latenz.

Jetzt (v2):

- **Eine `Client` pro App-Lifetime**: `CopilotBridge` lebt in
  `AppState.bridge`, wird beim ersten `chat_send` lazy erzeugt.
  Konfig-Mismatch (Endpoint/Model) triggert Recreation.
- **Eine `Session` pro Chat-Message**: `bridge.create_session()`
  liefert eine frische Session pro Request. Sessions kapseln den
  CLI-State (History, Tools, ...) pro Request lifecyclebar.
- **Streaming via `Session::subscribe()`**: Broadcast-Channel
  wird in der Event-Loop des SDK befuellt. Der Bridge-Stream-Loop
  konsumiert `assistant.message_delta`/`session.idle`/
  `session.error` und emittiert sie als Tauri-Events.

### Cancellation

`chat_cancel(request_id)` greift `session.abort()` auf der aktiven
Session (gehalten in `AppState.active_session: Mutex<Option<ActiveSession>>`
mit `Arc<tokio::sync::Mutex<Session>>` fuer Sharing). Der Stream-Loop
sieht darauf `session.idle`/`session.error` und emittiert
`chat.done`/`chat.error` mit dem bis dahin akkumulierten Content
(Partial-Response bei Cancel).

### Wichtige SDK-Reihenfolge

`subscribe()` MUSS **vor** `send()` aufgerufen werden, sonst verpasst
der Subscriber fruehe `assistant.message_delta`-Events (siehe SDK
`Session::subscribe` Doc). Der Bridge-Code respektiert diese
Reihenfolge.

### Event-Typen-Mapping

| SDK Event                         | Tauri Event  | Payload-Felder                             |
| --------------------------------- | ------------ | ------------------------------------------ |
| `assistant.message_delta`         | `chat_chunk` | `deltaContent` + `accumulated`             |
| `assistant.message`               | `chat_chunk` | Full-Content (Fallback fuer Non-Streaming) |
| `session.idle` / `assistant.idle` | `chat_done`  | (akkumulierten Content)                    |
| `session.error`                   | `chat_error` | `errorMessage`/`message`                   |

Nicht weitergeleitet: `assistant.message_start`, `assistant.streaming_delta`
(kumulative Bytes, redundant), `assistant.reasoning_delta`,
`assistant.tool_call_delta`, `tool.execution_*`, `session.start` etc.
TODO v3: separater Event-Channel fuer Tool-Progress-UI.

### Streaming aktivieren

Streaming ist **off by default** in SDK 1.0.8. Ohne explizites
`with_streaming(true)` auf `SessionConfig` senden die meisten
OpenAI-kompatiblen Provider (z. B. MiniMax M3) nur ein einziges
`assistant.message`-Event mit dem vollstaendigen Content statt
Token-Deltas. Der Bridge-Stream-Loop behandelt **beide** Varianten
(`assistant.message_delta` + `assistant.message`-Fallback), aber
fuer Echtzeit-Typewriter-Effekt ist `with_streaming(true)` Pflicht.

### Endpoint-Normalisierung (dedupe, nicht strip)

Der `dedupe_v1_suffix()`-Helper reduziert nur **doppelte**
`/v1/v1`-Suffixe auf eins. Ein einzelnes `/v1` (wie es viele
OpenAI-Provider als Teil ihrer Basis-URL erwarten, z. B. MiniMax M3)
bleibt unangetastet. Vorher wurde `/v1` immer abgeschnitten, was
bei diesen Providern zu 404 fuehrte. Unit-Tests in `bridge.rs::tests`
decken Happy-Path + Edge-Cases ab.

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

- **SDK-API-Stabilität**: SDK 1.x ist die aktuelle stabilie Major-Version.
    Wir pinnen auf `github-copilot-sdk = "1"`. Breaking Changes innerhalb
    der 1.x-Linie werden via SemVer-konforme Minor-Bumps signalisiert;
    die Bridge isoliert die SDK-Aufrufe, sodass Anpassungen lokal
    bleiben. Mitigation: Integrationslogik bleibt in `bridge.rs` +
    `process.rs` gekapselt.
- **Streaming-Granularität**: ✅ gelöst seit 2026-07-30. `Session::subscribe()`
    + `chat.chunk`/`chat.done`/`chat.error` Tauri-Events. Siehe
    `## Streaming-Architektur (v2)` oben.
- **Bundling-Strategie**: ✅ gelöst seit Commit `a1aa5bf` (SDK 1.0 +
    `bundled-cli` Default). Keine externe Sidecar-Binary mehr, kein
    `externalBin`, keine `bundle.resources`-Mappings. Release-Artefakt:
    `target/release/my-copilot.exe` (~100 MB, single-file).
- **Schema-Migration JSONL**: bei v1+ Schema-Changes für JSONL
  (rückwärtskompatibel via Default-Werte pro Feld).

## Quellen

- `github/copilot-sdk` (Rust-Variante) — offizielles SDK
- Tauri 2 Docs — Commands + Events — tauri.app
- `tokio::process::Command` — async Subprozess-Management
- JSONL-Pattern in Rust — `serde_json` + `tokio::fs::File`
- Tauri 2 Docs — Sidecar-Pattern (`externalBin`)