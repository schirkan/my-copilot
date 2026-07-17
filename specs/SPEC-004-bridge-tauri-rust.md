# SPEC-004 — Tauri-Rust Bridge (Copilot SDK Rust)

**Status:** Planungs-Phase, kein Code
**Datum:** 2026-07-17 (initial) / 2026-07-17 (rewrite: C# → Rust)
**Bezug:** SPEC-001 § Tech-Entscheidungen (Tauri-Rust Bridge) ·
SPEC-005 § IPC-Anbindung · `DECISIONS.md` § Architektur-Verschlankung

## Übersicht

Tauri-Rust ist nicht nur die App-Shell, sondern auch die **einzige
Bridge zwischen Frontend (CopilotKit React) und Copilot CLI**. Es
verwaltet den CLI-Subprozess, spricht JSON-RPC via Stdin/Stdout
(durch Copilot SDK Rust) und relayt Chunks via Tauri-Events an React.

**Wichtig:** Es wird **kein Port** für IPC geöffnet — weder HTTP noch
Named Pipe noch TCP. Alle Inter-Prozess-Kommunikation läuft über
OS-Pipes (Stdin/Stdout des Subprozesses).

## Komponenten-Stapel

```
React (CopilotKit)
  ↕ Tauri-IPC (Commands + Events, intern — kein Netzwerk)
Tauri-Rust (App-Shell + Bridge)
  ├── spawnt Subprozess
  └── JSON-RPC via Stdin/Stdout-Pipes (Copilot SDK Rust)
Copilot CLI (Node.js-App, embedded)
  ↕ HTTPS / SSE
OpenAI-kompatibler Endpoint
```

**Zwei Prozesse zur Laufzeit**: Tauri-Rust (App-Shell + Bridge) ·
Node.js+CLI.

## IPC-Architektur

```
React (CopilotKit)              Tauri-Rust (Bridge)              Copilot CLI
   │                                  │                                │
   │ invoke('chat.send')              │                                │
   ├─────────────────────────────────►│                                │
   │   (Tauri-Command)                │                                │
   │                                  │ JSON-RPC                       │
   │                                  │ {"method":"chat",...}          │
   │                                  ├───────────────────────────────►│
   │                                  │   (Stdin)                      │
   │                                  │                                │
   │                                  │ ◄─── Stream-Chunks ────────────│
   │                                  │   (Stdout)                     │
   │                                  │                                │
   │ onChunk → emit('chat.chunk')     │                                │
   │◄─────────────────────────────────│                                │
   │   (Tauri-Event)                  │                                │
```

**Zwei IPC-Layer**, beide ohne Netzwerk:

1. **React ↔ Tauri-Rust**: Tauri-IPC über interne Message-Pipes
   (Tauri-eigenes Protokoll, kein TCP).
2. **Tauri-Rust ↔ CLI**: Stdin/Stdout-Pipes des Subprozesses
   (JSON-RPC, **kein Port**, kein HTTP, kein Named Pipe).

## Subprozess-Management

Tauri-Rust spawned die CLI als Subprozess via `tokio::process::Command`:

```rust
use tokio::process::{Command, Stdio};
use std::path::Path;

pub struct CopilotCliProcess {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::process::ChildStdout,
}

impl CopilotCliProcess {
    pub async fn start(exe_dir: &Path) -> Result<Self, Error> {
        let node_exe = exe_dir.join("node").join(
            if cfg!(windows) { "node.exe" } else { "node" }
        );
        let cli_entry = exe_dir.join("copilot-cli").join("index.js");

        let mut child = Command::new(&node_exe)
            .arg(&cli_entry)
            .env("COPILOT_HOME", exe_dir.join("copilot-cli"))
            .env("NODE_PATH", exe_dir.join("copilot-cli").join("node_modules"))
            // Wichtig: PATH nicht ändern, sonst Kollision mit System-Node
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdin  = child.stdin.take().ok_or(Error::NoStdin)?;
        let stdout = child.stdout.take().ok_or(Error::NoStdout)?;
        // stderr → async in Log schreiben (siehe Implementierung)

        Ok(Self { child, stdin, stdout })
    }

    pub async fn wait_for_ready(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<(), Error> {
        // Lese erstes JSON-RPC-Response (Hello/Ready), max `timeout`
    }
}

impl Drop for CopilotCliProcess {
    fn drop(&mut self) {
        // kill_on_drop=true kümmert sich automatisch
    }
}
```

**Wichtig**: `kill_on_drop(true)` sorgt dafür, dass der Subprozess
sauber beendet wird, wenn Tauri-Rust ihn nicht mehr braucht (App-
Close oder manueller Restart).

## Copilot SDK Rust — Public API

```rust
use copilot_sdk::{CopilotClient, ChatRequest, ChatChunk};

pub struct CopilotService {
    client: CopilotClient,
    config: ByokConfig,
}

impl CopilotService {
    pub fn new(process: CopilotCliProcess, config: ByokConfig) -> Self {
        Self {
            client: CopilotClient::new(process),
            config,
        }
    }

    pub async fn chat_streaming(
        &self,
        user_message: String,
    ) -> impl tokio_stream::Stream<Item = ChatChunk> {
        self.client.chat_streaming(ChatRequest {
            message: user_message,
            model: self.config.model.clone(),
            api_key: self.config.api_key.clone(),
            base_url: self.config.endpoint.clone(),
            system_prompt: self.config.system_prompt.clone(),
            mcp_servers: self.config.mcp_servers.clone(),
        })
    }
}
```

> Hinweis: Die exakte API richtet sich nach dem finalen
> `github/copilot-sdk` Rust-Paket (Platzhalter). Bei Implementierung
> ggf. anpassen.

## IPC-Methoden (Tauri-Commands + Events)

| Methode           | Richtung        | Payload                                                |
|-------------------|-----------------|--------------------------------------------------------|
| `chat.send`       | Frontend → Rust | `{message: string}`                                    |
| `chat.cancel`     | Frontend → Rust | `{request_id: string}`                                 |
| `chat.chunk`      | Rust → Frontend | `{request_id, text: string}` (Stream)                  |
| `chat.done`       | Rust → Frontend | `{request_id, usage: TokenUsage}`                      |
| `chat.error`      | Rust → Frontend | `{request_id, error: string}`                          |
| `config.get`      | Frontend → Rust | `{}` → `{endpoint, model, systemPrompt, mcpServers}`   |
| `config.set`      | Frontend → Rust | `{endpoint, apiKey, model, systemPrompt, mcpServers}`  |
| `config.test`     | Frontend → Rust | `{endpoint, apiKey}` → `{ok, models}`                  |
| `process.health`  | Frontend → Rust | `{}` → `{cli_running, cli_ready}`                      |
| `process.restart` | Frontend → Rust | `{}` (Subprozess neu starten)                          |

Streaming-Pattern: Tauri-Event `chat.chunk` wird per
`app_handle.emit("chat.chunk", payload)` emittiert. React subscribt
via `listen("chat.chunk", ...)`.

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

| Fehler                         | Reaktion                              |
|--------------------------------|---------------------------------------|
| Node.js nicht gefunden         | Setup-Screen mit Hinweis              |
| Copilot CLI Crashed            | Auto-Restart (max 3 Versuche), dann User-Notification |
| Stdin/Stdout-Pipe broken       | Auto-Restart, Frontend bekommt `process.restart`-Event |
| BYOK-Endpoint 401              | „API-Key ungültig" → Settings-Dialog   |
| BYOK-Endpoint 429              | Exponential Backoff + Fallback-Modell  |
| BYOK-Endpoint Network-Error    | Retry mit User-Bestätigung             |
| Tauri-IPC Timeout              | User-Notification „Bridge antwortet nicht" |

## Offene Punkte

- **Sidecar-Lifecycle**: Tauri 2 `externalBin` (statisch konfiguriert,
  automatischer Lifecycle) vs. manuelle `tokio::process::Command`-
  Verwaltung (dynamischer, eigener Restart-Loop). Für Node.js-
  Subprozess eher manuelle Verwaltung sinnvoll, da eigenes Restart-
  Handling + Health-Checks gebraucht werden.
- **Copilot SDK Rust — API-Stabilität**: SDK noch jung, API kann
  sich noch ändern. Mitigation: in einer Bridge-Schicht gekapselt,
  Refactor-Aufwand begrenzt.
- **Streaming-Granularität**: Wie klein sind CLI-Chunks? Per Token,
  per Satz, per Message? Bei Implementierung evaluieren.
- **Schema-Migration JSONL**: bei v1+ Schema-Changes für JSONL
  (rückwärtskompatibel via Default-Werte pro Feld).

## Quellen

- `github/copilot-sdk` (Rust-Variante) — offizielles SDK
- Tauri 2 Docs — Commands + Events — tauri.app
- `tokio::process::Command` — async Subprozess-Management
- JSONL-Pattern in Rust — `serde_json` + `tokio::fs::File`
- Tauri 2 Docs — Sidecar-Pattern (`externalBin`)