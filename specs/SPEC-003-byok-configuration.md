# SPEC-003 — BYOK Configuration

**Status:** Planungs-Phase
**Datum:** 2026-07-17
**Bezug:** SPEC-001 § BYOK · VSCode 1.129 Release Notes (15.07.2026)

## Ziel

`My Copilot` unterstützt **Bring Your Own Key (BYOK)** für jeden
OpenAI-kompatiblen Endpoint. Der User trägt API-Key + Endpoint einmalig
in der App ein, App speichert verschlüsselt. Kein GitHub-Copilot-Abo
nötig.

## Unterstützte Endpoints

| Endpoint-Typ             | Beispiel-URL                              |
|--------------------------|-------------------------------------------|
| Azure OpenAI             | `https://<resource>.openai.azure.com`     |
| `api.openai.com`         | `https://api.openai.com/v1`               |
| Self-hosted vLLM         | `http://localhost:8000/v1`                |
| Self-hosted LM Studio    | `http://localhost:1234/v1`                |
| OpenRouter               | `https://openrouter.ai/api/v1`            |
| Together / Groq / Portkey | jeweilige Docs                            |

Alle funktionieren via Custom-`baseURL`-Parameter im OpenAI-kompatiblen
Client (Copilot CLI / Microsoft Agent Framework / etc.).

## config.json Format

```json
{
  "version": 1,
  "endpoint": {
    "type": "azure-openai",
    "baseUrl": "https://my-resource.openai.azure.com",
    "apiKeyCipher": "AQAAANCMnd8BfGdCAA...",
    "deploymentName": "gpt-4o"
  },
  "model": {
    "default": "gpt-4o",
    "fallback": "gpt-4o-mini"
  },
  "dataDir": "./data",
  "logLevel": "info",
  "telemetry": false
}
```

**Felder:**
- `endpoint.type`: `azure-openai` | `openai` | `openai-compatible`
- `endpoint.baseUrl`: Custom-Endpoint-URL (alle OpenAI-kompatiblen)
- `endpoint.apiKeyCipher`: DPAPI-verschlüsselt (CurrentUser-Scope)
- `endpoint.deploymentName`: nur für Azure OpenAI relevant
- `model.default`: Standard-Modell für Chat-Completions
- `model.fallback`: Optional, bei Rate-Limits o.ä.
- `dataDir`: relativ zu exe (immer `./data`)
- `logLevel`: `trace` | `debug` | `info` | `warn` | `error`
- `telemetry`: per Default aus (Privacy)

## DPAPI-Verschlüsselung (Windows)

```csharp
using System.Security.Cryptography;

// Verschlüsseln
var plainBytes = Encoding.UTF8.GetBytes(apiKey);
var cipherBytes = ProtectedData.Protect(
    plainBytes,
    optionalEntropy: null,
    scope: DataProtectionScope.CurrentUser);
File.WriteAllBytes("config.json", cipherBytes);

// Entschlüsseln
var cipherBytes = File.ReadAllBytes("config.json");
var plainBytes = ProtectedData.Unprotect(
    cipherBytes,
    optionalEntropy: null,
    scope: DataProtectionScope.CurrentUser);
var apiKey = Encoding.UTF8.GetString(plainBytes);
```

**Wichtig**: DPAPI ist an den Windows-User-Account gebunden. Die Config
ist nicht portabel über User-Accounts hinweg — aber genau das wollen
wir bei einer BYOK-Config ja auch nicht.

## Erstlauf-Flow

1. App startet, sieht keine `config.json`.
2. App zeigt **Config-Screen im Frontend** (CopilotKit React):
   - Endpoint-URL
   - API-Key
   - Modell-Auswahl (Dropdown mit Vorschlägen je nach Endpoint-Typ)
   - Test-Button: schickt `GET /v1/models` an den Endpoint, prüft Auth.
3. User trägt Daten ein, klickt **Speichern & Testen**.
4. App schreibt `apiKey` im Klartext in `config.json` (v1, siehe §
   Encryption). In v2 wird DPAPI/OS-Keychain vor dem Schreiben
   verschlüsselt (siehe DECISIONS.md § Config-Storage-v1-plaintext).
5. App startet den normalen Chat-Workflow.

## Update-Flow (Endpoint-Wechsel)

User kann Endpoint/Key jederzeit über Settings-Dialog ändern. Alte
`config.json` wird durch neue ersetzt (nach erfolgreichem Test).

## Auth-Alternative: GitHub-OAuth

Theoretisch könnte die App auch ohne BYOK laufen, via GitHub-OAuth (wie
VSCode-1.129-Copilot-Agent-Host). **Wir unterstützen das NICHT in v1**,
weil:

- OAuth-Browser-Redirect-Flow in Tauri-WebView komplex
- Zielgruppe hat eh eigene API-Keys (Azure, Self-hosted)
- BYOK ist privacy-freundlicher (kein GitHub-Server-Traffic)

→ Falls später gewünscht: OAuth als zusätzliche Auth-Option in v2.

## Quellen

- VSCode 1.129 Release Notes § BYOK
- Microsoft Learn — `ProtectedData.Protect`
- Copilot SDK — BYOK Configuration-Object