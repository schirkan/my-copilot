# SPEC-004 — Backend (C# · Copilot SDK .NET)

**Status:** Planungs-Phase
**Datum:** 2026-07-17
**Bezug:** SPEC-001 § C# Backend · SPEC-002 § Pfad-Resolution

## Verantwortlichkeit

Das C#-Backend ist die **einzige Brücke zwischen Frontend
(Tauri-WebView) und Copilot CLI**. Es:

1. Verwaltet den **Node.js+Copilot-CLI-Subprozess** (Start, Restart,
   Kill).
2. Spricht via **JSON-RPC über Stdin/Stdout** mit der CLI (durch Copilot
   SDK .NET).
3. Übersetzt **BYOK-Config** (siehe SPEC-003) in CLI-Auth-Header.
4. Stellt dem Frontend ein **stabiles IPC-API** zur Verfügung
   (Tauri-Rust → C# via Sidecar-Pattern).
5. Persistiert **Chat-History & State** in `./data/` (SQLite).

## IPC-Architektur

```
React (CopilotKit)                C# Backend                 Copilot CLI
   │                                │                          │
   │ sendMessage(text)              │                          │
   ├──────────────────────────────►│                          │
   │   (via Tauri-Command)          │                          │
   │                                │ JSON-RPC:                │
   │                                │ {"method":"chat",...}    │
   │                                ├─────────────────────────►│
   │                                │                          │
   │                                │ ◄─── SSE Stream ──────── │
   │                                │                          │
   │ onChunk(text)                  │                          │
   │◄──────────────────────────────│                          │
   │   (via Tauri-Event)            │                          │
```

## Subprozess-Management

```csharp
public class CopilotCliProcess : IDisposable {
    private Process? _process;
    private readonly ILogger _log;
    private readonly string _nodeExe;
    private readonly string _cliEntry;

    public async Task StartAsync(CancellationToken ct) {
        var psi = new ProcessStartInfo {
            FileName = _nodeExe,
            Arguments = $"\"{_cliEntry}\"",
            UseShellExecute = false,
            RedirectStandardInput  = true,
            RedirectStandardOutput = true,
            RedirectStandardError  = true,
            CreateNoWindow         = true,
        };
        psi.Environment["COPILOT_HOME"] = Path.GetDirectoryName(_cliEntry);
        psi.Environment["NODE_PATH"] = Path.Combine(
            Path.GetDirectoryName(_cliEntry)!, "node_modules");

        _process = Process.Start(psi)!;
        _process.ErrorDataReceived += (s, e) => _log.Warn(e.Data);
        _process.BeginErrorReadLine();

        // Health-Check: warte auf "ready" innerhalb 10s
        await WaitForReadyAsync(TimeSpan.FromSeconds(10), ct);
    }

    public void Dispose() {
        if (_process is { HasExited: false }) {
            _process.Kill(entireProcessTree: true);
            _process.Dispose();
        }
    }
}
```

## Copilot SDK .NET — Public API

```csharp
using GitHub.Copilot.Sdk;  // (Platzhalter-Paketname)

public class CopilotService {
    private readonly CopilotClient _client;
    private readonly ByokConfig _config;

    public CopilotService(CopilotCliProcess proc, ByokConfig config) {
        _client = new CopilotClient(proc);
        _config = config;
    }

    public async IAsyncEnumerable<string> ChatAsync(
        string userMessage,
        CancellationToken ct
    ) {
        await foreach (var chunk in _client.ChatStreamingAsync(
            message: userMessage,
            model: _config.Model,
            apiKey: _config.ApiKey,
            baseUrl: _config.Endpoint,
            ct: ct
        )) {
            yield return chunk;
        }
    }
}
```

## IPC-Methoden (Tauri-Rust → C# Sidecar)

| Methode             | Richtung         | Payload                              |
|---------------------|------------------|--------------------------------------|
| `chat.send`         | Frontend → C#    | `{message: string}`                  |
| `chat.cancel`       | Frontend → C#    | `{requestId: string}`                |
| `chat.chunk`        | C# → Frontend    | `{requestId, text: string}` (Stream) |
| `chat.done`         | C# → Frontend    | `{requestId, usage: {tokens:...}}`   |
| `chat.error`        | C# → Frontend    | `{requestId, error: string}`         |
| `config.get`        | Frontend → C#    | `{}` → `{endpoint, model, ...}`      |
| `config.set`        | Frontend → C#    | `{endpoint, apiKey, model}`          |
| `config.test`       | Frontend → C#    | `{endpoint, apiKey}` → `{ok, models}`|
| `process.health`    | Frontend → C#    | `{}` → `{nodeRunning, cliReady}`     |
| `process.restart`   | Frontend → C#    | `{}` (Node.js+CLI neu starten)       |

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
Für v1 mit ~100–1000 Sessions OK. Falls später nötig: Sidecar-Index-File
oder Migration zu SQLite.

## Fehlerbehandlung

| Fehler                         | Reaktion                              |
|--------------------------------|---------------------------------------|
| Node.js nicht gefunden         | Setup-Screen mit Hinweis              |
| Copilot CLI Crashed            | Auto-Restart (max 3 Versuche), dann User-Notification |
| BYOK-Endpoint 401              | „API-Key ungültig" → Settings-Dialog  |
| BYOK-Endpoint 429              | Exponential Backoff + Fallback-Modell |
| BYOK-Endpoint Network-Error    | Retry mit User-Bestätigung            |
| WebView-IPC Timeout            | User-Notification „Backend antwortet nicht" |

## Offene Punkte

- **Tauri-Sidecar-Setup**: Tauri-Config `externalBin` für C#-exe, oder
  C# als Service? — siehe SPEC-001 § Offene Punkte
- **Streaming-Protokoll**: SSE vom Copilot CLI → C# → Tauri-Events?
  Oder einfacher: Polling?
- **Schema-Migration**: bei v1+ Schema-Changes für SQLite

## Quellen

- `github/copilot-sdk` (.NET-Variante)
- Microsoft Learn — `Process.Start` / Subprozess-Management
- SQLite mit .NET — `Microsoft.Data.Sqlite`
- Tauri 2 — Sidecar-Pattern