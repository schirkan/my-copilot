# Manuelles End-to-End-Smoke-Test-Protokoll (v1)

Dieses Protokoll validiert alle SPEC-001..006-Anforderungen durch manuelle
Tests. Es ist kein automatisiertes Test-Framework — die Schritte werden per
Hand ausgeführt und das Ergebnis in den Checkboxen dokumentiert.

## Pre-requisites

- [ ] `MyCopilot.exe` gebaut via `cargo tauri build` (oder vorgefertigter Build vorhanden)
- [ ] Frischer, leerer Ordner für den Test (kein `config.json`, kein `data/`)
- [ ] OpenAI-kompatibler Endpoint erreichbar (z. B. `https://api.openai.com/v1` oder self-hosted vLLM auf `http://localhost:8000/v1`)
- [ ] Gültiger API-Key für diesen Endpoint

## Test-Sections

### Test 1 — App-Start (Erstlauf, kein `config.json`)

- [ ] Doppelklick auf `MyCopilot.exe`
- [ ] App-Fenster öffnet sich (Titel "My Copilot", Default-Größe 1024×768)
- [ ] Settings-Dialog erscheint automatisch im Erstlauf-Mode
- [ ] Cancel-Button ist disabled oder nicht vorhanden (Erstlauf blockiert)
- [ ] Tab-Wechsel Connection → Behavior → Tools funktioniert
- [ ] Schließen des Fensters → Prozess beendet sauber (kein Zombie-CLI)

### Test 2 — BYOK-Setup mit echtem Endpoint

- [ ] Connection-Tab → Endpoint-URL eintragen (z. B. `https://api.openai.com/v1`)
- [ ] API-Key eintragen (Password-Feld, wird maskiert)
- [ ] "Test Endpoint" klicken → Modell-Liste erscheint in der Datalist
- [ ] Test-Result zeigt ✓ "N Models gefunden"
- [ ] Behavior-Tab → System-Prompt eintragen (z. B. "Du bist ein hilfreicher Coding-Assistent…")
- [ ] Tools-Tab → MCP-Server hinzufügen:
  - Name: `filesystem`
  - Transport: `stdio`
  - Command: `npx -y @modelcontextprotocol/server-filesystem /tmp`
- [ ] Save klicken
- [ ] Dialog schließt sich automatisch
- [ ] Hauptansicht (Chat-UI mit Sidebar) wird sichtbar
- [ ] Header zeigt Endpoint + Model: `https://api.openai.com/v1 · gpt-4o`
- [ ] `config.json` im exe-Verzeichnis prüfen:
  ```bash
  cat config.json
  ```
  Erwartet: `api_key` im Klartext, `system_prompt`, `mcp_servers` mit filesystem-Eintrag

### Test 3 — Chat-Workflow

- [ ] Frage eingeben: "Was ist 2+2?"
- [ ] Enter drücken (Shift+Enter für newline)
- [ ] Senden klicken (oder Enter)
- [ ] User-Message erscheint als Bubble rechts
- [ ] Loading-Spinner oder "… denke nach …" während Wartezeit
- [ ] Assistant-Antwort erscheint (non-streaming v1: komplette Antwort auf einmal)
- [ ] Assistant-Bubble links mit Antwort "4" oder ähnlich
- [ ] Input-Feld wird geleert nach Send
- [ ] Auto-Scroll zur neuesten Message
- [ ] Sidebar zeigt neue Session mit title "Was ist 2+2?" (gekürzt auf 50 chars)
- [ ] Session zeigt `1 msgs · gpt-4o` (oder ähnlich)
- [ ] `data/sessions/{session-id}.jsonl` prüfen:
  ```bash
  ls data/sessions/
  cat data/sessions/{uuid}.jsonl
  ```
  Erwartet: 2 JSON-Zeilen (User + Assistant), je eine Message mit korrektem Schema

### Test 4 — History-Persistenz

- [ ] Weitere 2-3 Messages in der gleichen Session senden
- [ ] Session in Sidebar anklicken → Console-Log zeigt `Loaded N messages`
- [ ] (v1.1 TODO: Messages werden im Hauptfenster angezeigt; v1: log-only)
- [ ] `data/sessions/{session-id}.jsonl` enthält jetzt 5-7 Messages (append-only)
- [ ] Jede Zeile ist gültiges JSON mit `id`, `request_id`, `role`, `content`, `ts`, `model`, `tokens`

### Test 5 — Settings ändern (Endpoint-Switch)

- [ ] "Settings" klicken (oben rechts)
- [ ] Settings-Dialog öffnet sich mit aktuellen Werten aus `config.json`
- [ ] Endpoint auf anderen Provider ändern (z. B. `http://localhost:8000/v1` für vLLM)
- [ ] API-Key ändern
- [ ] Save → Dialog schließt
- [ ] `config.json` prüfen: neue Endpoint-Werte persistent (atomic write via Temp+Replace)
- [ ] Alte `config.json.tmp` falls vorhanden wurde gelöscht (oder überschrieben)

### Test 6 — App-Neustart mit config.json

- [ ] App schließen (Fenster-Close)
- [ ] Task-Manager prüfen: keine Zombie-Prozesse (kein `MyCopilot.exe` mehr aktiv)
- [ ] App neu starten
- [ ] Settings-Dialog erscheint NICHT (Config wird automatisch geladen)
- [ ] Header zeigt neuen Endpoint
- [ ] Sidebar zeigt vorherige Sessions aus JSONL

### Test 7 — Process-Health & Restart

- [ ] Während App läuft: DevTools öffnen (z. B. via Tauri-Plugin oder `tauri::Manager::devtools`)
- [ ] Console: `await __TAURI__.core.invoke("process_health")`
- [ ] Response zeigt `{cli_running: true, cli_ready: true, bridge_initialized: true|false}`
- [ ] Eine Nachricht senden → währenddessen erneut `process_health` → `bridge_initialized: true` (oder `false` nach Send)
- [ ] Console: `await __TAURI__.core.invoke("process_restart")` → kein Fehler
- [ ] Nächste Message funktioniert (neue Bridge wurde gespawnt)

### Test 8 — Edge-Cases

- [ ] **API-Key mit Sonderzeichen**: z. B. Key mit `+`, `/`, `=` (Base64-Padding)
  - Save lädt → nächste Message funktioniert
- [ ] **Sehr langer System-Prompt** (>10 KB):
  - Save lädt → wird in config.json korrekt gespeichert
  - Nächste Message funktioniert (CLI akzeptiert den Prompt)
- [ ] **Endpoint nicht erreichbar**:
  - Endpoint auf `http://localhost:1` (ungültig) setzen
  - Save lädt
  - Test-Endpoint klicken → ✗ Fehler nach Timeout
- [ ] **Ungültiger Endpoint-URL-Format**:
  - Endpoint auf `not-a-url` setzen
  - Test-Endpoint → ✗ Fehler (URL-Parse-Error)
- [ ] **Leere Messages**:
  - Nur Leerzeichen eingeben + Senden → sollte disabled sein (Button)
- [ ] **App während Streaming schließen**:
  - Message senden, sofort Fenster schließen
  - Task-Manager: kein Zombie-Prozess (kill_on_drop=true)

## Acceptance-Kriterien

- [ ] Alle 8 Test-Sections bestanden
- [ ] Keine Crashes oder unbehandelte Errors
- [ ] Logs zeigen keine ERROR-Level-Einträge
- [ ] `data/sessions/` enthält mindestens eine valide JSONL-Datei

## Test-Endpoints (Empfehlung für vollständige Abdeckung)

- [ ] OpenAI (`https://api.openai.com/v1`)
- [ ] Azure OpenAI (`https://{resource}.openai.azure.com/openai/deployments/{deployment}/v1`)
- [ ] Self-hosted vLLM (`http://localhost:8000/v1`)
- [ ] OpenRouter (`https://openrouter.ai/api/v1`)

## Bekannte Limitierungen (v1)

- **Kein Token-für-Token-Streaming**: `chat_send` ist non-streaming. Die Antwort kommt als kompletter String auf einmal, nicht chunk-weise. Echtes Streaming kommt in v1.1 via Tauri-Events (`chat_chunk`).
- **History-Load zeigt nur log**: Bei Sidebar-Klick auf eine Session wird nur `console.info` geloggt. Die Messages werden nicht im Hauptfenster angezeigt. Das kommt in v1.1.
- **Kein Markdown-Rendering**: Assistant-Responses werden als Plain-Text angezeigt. `react-markdown` Integration kommt in v1.1.
- **Tool-Calls nur als Badge-Stubs**: MCP-Server-Tool-Calls werden in v1 nicht im UI dargestellt, sondern nur an die CLI durchgereicht. Tool-Definitionen via `useCopilotAction` kommen in v2 mit vollständiger MCP-Integration.

## Test-Protokoll abschließen

- [ ] Alle Tests bestanden → Test-Protokoll signieren mit Datum
- [ ] Etwaige Failures im Issue-Tracker dokumentieren (GitHub Issues)
- [ ] Bei Major-Failures: Rollback auf letzten funktionierenden Commit
