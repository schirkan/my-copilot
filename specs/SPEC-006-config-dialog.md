# SPEC-006 — Config Dialog (Settings UI)

**Status:** Planungs-Phase
**Datum:** 2026-07-17
**Bezug:** SPEC-003 § BYOK Config · SPEC-004 § IPC · SPEC-005 § Frontend ·
`DECISIONS.md` (BYOK-only, JSONL-Persistenz)

## Übersicht / Zweck

Die App braucht einen zentralen Konfigurations-Dialog für alle
User-Einstellungen, die zur Laufzeit änderbar sind:

- **LLM-Connection**: API Key, Base URL, Endpoint-Typ, Modell
- **Verhalten**: System Prompt
- **Tools**: MCP Servers

Wird sowohl beim **Erstlauf** (wenn keine `config.json` existiert) als
auch jederzeit aus den **Settings** aufrufbar sein.

## UI-Struktur

```
ConfigDialog (Modal)
├── Tabs / Sections
│   ├── [Connection]
│   │   ├── Endpoint-Typ        (Dropdown)
│   │   ├── API Base URL        (Text)
│   │   ├── API Key             (Password)
│   │   ├── Modell              (Dropdown + Freitext-Fallback)
│   │   └── [Test Endpoint]     (Button)
│   ├── [Behavior]
│   │   ├── System Prompt       (Textarea, mehrzeilig, monospace)
│   │   └── [Reset auf Default] (Button)
│   └── [Tools / MCP]
│       ├── MCP-Server-Liste    (Cards oder Rows)
│       │   pro Eintrag: Name, Transport, Endpoint/Command,
│       │                 Enabled-Toggle, [Edit] [Remove]
│       └── [+ Add MCP Server]  (Button)
└── Footer
    ├── [Abbrechen]   (verwirft Änderungen)
    └── [Speichern]   (schreibt config.json + ggf. App-Restart)
```

## Configuration Items

### 1) API Key

- **Type:** Password-Input
- **Storage:** DPAPI-verschlüsselt in `config.json`
  (`endpoint.apiKeyCipher`)
- **Required:** Ja
- **Validation:** Nicht leer + Endpoint-Test
- **Decision:** BYOK-only, kein GitHub-OAuth (siehe `DECISIONS.md`).

### 2) API Base URL

- **Type:** Text-Input
- **Examples:**
  - `https://my-resource.openai.azure.com` (Azure)
  - `https://api.openai.com/v1` (OpenAI)
  - `http://localhost:8000/v1` (self-hosted)
  - `https://openrouter.ai/api/v1` (OpenRouter)
- **Storage:** Klartext in `config.json` (`endpoint.baseUrl`)
- **Required:** Ja
- **Validation:** URL-Pattern + Endpoint-Test

### 3) Endpoint-Typ

- **Type:** Dropdown
- **Options:** `azure-openai` · `openai` · `openai-compatible`
- **Storage:** `config.json` (`endpoint.type`)
- **Required:** Ja
- **Note:** Bei `azure-openai` zusätzlich `deploymentName` Pflicht.

### 4) Modell

- **Type:** Dropdown (mit Freitext-Fallback)
- **Storage:** `config.json` (`model.default`, optional `model.fallback`)
- **Required:** Ja
- **UX:** Dropdown wird mit Modellen aus Endpoint-Test-Response
  befüllt (wenn verfügbar), sonst Freitext.

### 5) System Prompt (NEU)

- **Type:** Textarea (mehrzeilig, optional monospace)
- **Default:** leer — Copilot CLI hat eigenen Default.
- **Storage:** Klartext in `config.json` (`systemPrompt`)
- **Required:** Nein
- **Behavior:** Wird beim Start der Copilot CLI übergeben (genauer
  Mechanismus siehe Open Point „MCP/SystemPrompt-Übergabe").
- **Reset:** „Reset auf Default" stellt leeren String wieder her.
- **Scope:** Global — wirkt auf alle Sessions.
- **Open Point:** Soll ein kleiner Default-Prompt mitgeliefert werden
  (z. B. „Du bist ein hilfreicher Coding-Assistent...")? Oder lieber
  leer lassen und dem User die volle Hoheit geben?

### 6) MCP Servers (NEU)

- **Type:** Liste editierbarer Einträge
- **Pro Eintrag:**

| Feld        | Type   | Required       | Notes                                          |
|-------------|--------|----------------|------------------------------------------------|
| `name`      | string | ja             | Identifier, eindeutig in der Liste             |
| `transport` | enum   | ja             | `stdio` / `sse` / `http`                       |
| `command`   | string | ja bei `stdio` | Binary-Pfad + Args, z. B. `npx -y @mcp/server-filesystem` |
| `url`       | string | ja bei sse/http| Endpoint-URL des MCP-Servers                   |
| `env`       | object | nein           | Env-Vars für den Prozess                       |
| `enabled`   | bool   | ja             | Default `true`                                 |

- **Storage:** Klartext in `config.json` (`mcpServers` Array)
- **Behavior:** Werden beim Start der Copilot CLI registriert
  (genauer Mechanismus siehe Open Point „MCP/SystemPrompt-Übergabe").
- **Validation:** Pro Eintrag: `name` nicht leer, `transport`-
  spezifische Pflichtfelder gefüllt, kein Duplikat-`name`.
- **Edit-UI:** Sub-Dialog mit den 6 Feldern, Validierung inline.
- **UX:** Disabled-Toggle pro Eintrag, ohne den Eintrag zu löschen.

## Validation Flow

```
User öffnet Dialog
  → Lade aktuelle config.json (falls vorhanden)
  → Felder mit aktuellen Werten befüllen

User klickt [Test Endpoint]:
  → Backend: GET /v1/models (oder äquivalent) mit Endpoint+Key
  → Ergebnis: ✓ "N Models gefunden" oder ✗ Fehlertext
  → Bei Erfolg: Modell-Dropdown mit gefundenen Modellen befüllen

User klickt [Speichern]:
  → Validate alle Felder (Pflichfelder, URL-Pattern, MCP-Konsistenz)
  → Validation-Fehler → Inline-Anzeige, Speichern blockiert
  → OK → Backend schreibt config.json
         (DPAPI für API-Key, Rest Klartext inkl. systemPrompt +
          mcpServers), speichert und restartet App (siehe Open Point
          „Hot-Reload vs. Restart")
```

## Persistenz

- **Storage:** `config.json` im exe-Verzeichnis (siehe SPEC-002 § Folder-Layout)
- **Encryption:** DPAPI für `endpoint.apiKeyCipher` (CurrentUser-Scope,
  siehe DECISIONS.md § Auth-Flow)
- **Rest:** Klartext (Base URL, Endpoint-Typ, Modell, System Prompt,
  MCP Servers)
- **Wichtig:** Chat-History bleibt in JSONL (DECISIONS.md §
  Persistenz-Format) — Config ist separates File.

## IPC-Methoden

Erweitert SPEC-004 § IPC-Methoden:

| Methode       | Richtung         | Payload                                      | Notes                          |
|---------------|------------------|----------------------------------------------|--------------------------------|
| `config.get`  | Frontend → C#    | `{}` → `{endpoint, model, systemPrompt, mcpServers}` | Erweitert um neue Felder |
| `config.set`  | Frontend → C#    | `{endpoint, apiKey, model, systemPrompt, mcpServers}`  | Erweitert             |
| `config.test` | Frontend → C#    | `{endpoint, apiKey}` → `{ok, models}`        | Unverändert                    |

> Hinweis: Schema-Erweiterung von `config.json` (`systemPrompt`,
> `mcpServers`) muss in SPEC-003 § config.json nachgezogen werden —
> ist hier nicht doppelt dokumentiert, sondern nur referenziert.
> Follow-up-Commit.

## UX-Details

- **Erstlauf:** Dialog öffnet automatisch beim ersten Start, kein
  Cancel möglich — App kann ohne Config nicht starten.
- **Settings:** Dialog öffnet aus Sidebar/Settings-Button, Cancel
  verwirft ungespeicherte Änderungen (mit Warnung), Save committed.
- **Tab-Wechsel ohne Save:** Warnung „Ungespeicherte Änderungen —
  trotzdem wechseln?" (optional für v1, kann auch ohne sein).
- **Loading-States:** Test-Endpoint zeigt Spinner, Save-Button
  disabled während Test läuft.
- **Fehler-Anzeige:** Inline unter dem Feld + persistenter Banner oben
  für globale Fehler.

## Offene Punkte

- **System-Prompt-Default**: leer vs. mitgelieferter Default-Prompt?
- **MCP/SystemPrompt-Übergabe**: Wie genau übergibt die aktuelle
  Copilot-CLI / Copilot-SDK System-Prompt und MCP-Server-Config?
  (Stdio-Command-Spec für MCP, `--system-prompt`-Flag, `--mcp-config
  <file>` o. ä.) — bei Implementierung mit der jeweils aktuellen
  CLI-Version abgleichen.
- **MCP-Server-Bundling**: Soll die App v1 mit ein paar Standard-MCP-
  Servern ausgeliefert werden (z. B. `server-filesystem`,
  `server-github`, `server-fetch`)? Oder rein User-Konfiguration?
  Bundle-Size-Implikation (jeder MCP-Server ist eine Node-App mit
  eigenen Deps, ~10–50 MB).
- **Hot-Reload vs. Restart**: Werden System Prompt / MCP Servers zur
  Laufzeit wirksam oder erst nach App-Restart? Empfehlung v1:
  Restart (einfacher, weniger Edge-Cases). Hot-Reload wäre v2.
- **Sprache der UI**: Deutsch oder Englisch? Spec-005 hat das schon als
  Open Point.
- **Per-Session-Override**: Soll System Prompt und/oder Modell pro
  Session überschreibbar sein, oder nur global?
- **MCP-Server-Templates**: Soll die UI vorgefertigte Templates für
  beliebte MCP-Server anbieten (1-Klick-Add)?

## Spec-Follow-ups

- **SPEC-003 § config.json**: Schema um `systemPrompt` (string) und
  `mcpServers` (Array) erweitern.
- **SPEC-004 § IPC-Methoden**: Tabelle auf die erweiterte
  Payload-Shape anpassen (siehe oben).
- **SPEC-002 § Folder-Layout**: Mini-Stale aus dem JSONL-Decision
  bereinigen (`chat-history.db` → `sessions/{session-id}.jsonl`).