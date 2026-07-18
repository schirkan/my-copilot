# Decisions

Architektur- und Projekt-Entscheidungen mit Datum, Begründung und
verworfenen Alternativen. Wird on-demand geladen — nicht im Standard-
Workspace-Context.

---

## 2026-07-17 — Initial Setup

### Lizenz

- **Decision:** MIT (siehe `LICENSE`).
- **Reason:** Alle direkten Deps (Tauri, GitHub Copilot SDK, CopilotKit,
  React, Node.js, .NET) sind bereits MIT. Lizenz-Konsistenz, kurz,
  sehr permissiv, kein Contributor-Agreement nötig.
- **Verworfen:** Apache-2.0 (expliziter Patent-Grant, formaler, längerer
  Text) — für Personal-Tool nicht nötig, kein Contributor-Pool.
- **Source:** Commit `05ff6aa`, `LICENSE`

### Node.js als Build- und Runtime-Component

- **Decision:** Node.js v22+ ist **sowohl** Build-Tool (Vite, npm) als
  auch Runtime-Component (Copilot CLI läuft darauf). Im App-Bundle
  embedded als `node/node.exe`.
- **Reason:** Copilot CLI ist native Node.js-Anwendung. End-User
  installiert kein Node.js — embedded Binary reicht.
- **Verworfen:** Node.js nur als Build-Tool, Copilot CLI durch
  Alternative ersetzen — nicht möglich, CLI ist native Node.
- **Trade-off:** +30 MB Bundle-Size für Zero-Install-UX.
- **Reference:** SPEC-001 § Embedded Node.js, SPEC-002 § Folder-Layout

### Auth-Flow: BYOK-only

- **Decision:** Nur BYOK für v1, **kein** GitHub-OAuth.
- **Reason:** Zielgruppe hat eigene API-Keys (Azure, self-hosted);
  OAuth-Browser-Redirect in Tauri-WebView ist komplex; BYOK ist
  privacy-freundlicher (kein GitHub-Server-Traffic).
- **Verworfen:** GitHub-OAuth (Spec-003 § Auth-Alternative) — frühestens
  v2, wenn überhaupt.
- **Reference:** SPEC-003 § Auth-Alternative

### Update-Mechanismus

- **Decision:** **Kein** Update-Mechanismus für v1.
- **Reason:** Personal-Tool, „portable Folder" wird bei Bedarf manuell
  ersetzt (Re-Download ZIP oder `git pull`). Auto-Update-Framework wäre
  Komplexität ohne klaren Mehrwert für v1.
- **Verworfen:** Custom-Auto-Updater (Squirrel, Sparkle) — Overhead,
  frühestens v2 wenn überhaupt.
- **Trade-off:** User muss selbst updaten — aber Folder bleibt kopierbar
  und Stateless.
- **v3-TODO:** Auto-Update-Mechanismus (z. B. GitHub-Releases-Checker
  mit manueller Bestätigung, oder Squirrel/Sparkle-Wrapper). Erst
  sinnvoll wenn mehrere Versionen zirkulieren und User den
  Update-Pfad nicht jedes Mal manuell prüfen sollen.

### Code-Signing

- **Decision:** **Ohne** Code-Signing für v1.
- **Reason:** SmartScreen-Warning beim Erststart ist lästig, aber für
  Personal-Tool akzeptabel. EV-Code-Signing-Zertifikat (~300 €/Jahr)
  lohnt erst bei öffentlichem Release mit Fremd-Usern.
- **Verworfen:** Self-Signed (Vertrauens-Chaos), EV-Cert (Kosten/Nutzen
  für v1 zu hoch).
- **Reference:** SPEC-002 § Offene Punkte
- **v3-TODO:** EV-Code-Signing-Zertifikat (Certifikats-Provider
  wie DigiCert, Sectigo, GlobalSign). Implementierung: `signtool.exe`
  in der CI-Pipeline nach `cargo tauri build`. Erwartete Schritte:
  Certifikat kaufen → in GitHub-Secrets als Base64 speichern →
  GitHub-Actions-Workflow erweitern → signtool-Aufruf hinzufügen →
  SmartScreen-Warning verschwindet bei Installation.

### Distribution-Channel

- **Decision:** GitHub Releases für v1.
- **Reason:** Kostenlos, ZIP-Download, passt zum Repo, keine eigene
  Infrastruktur nötig.
- **Verworfen:** Eigener Server / CDN — Overhead für v1, frühestens v2.
- **Reference:** SPEC-002 § Distribution

### Persistenz-Format

- **Decision:** JSONL, eine Datei pro Session unter
  `./data/sessions/{session-id}.jsonl`. Eine JSON-Zeile pro Message mit
  Schema `{id, request_id, role, content, ts, model, tokens}`.
- **Reason:** Kein Native-Dep (kein `Microsoft.Data.Sqlite`), kein
  Schema-Migration-Overhead, human-readable und easy Backup/Inspect,
  append-only robust gegen Teil-Schreibfehler, per-Session-Files für
  einfaches Löschen/Restore einzelner Sessions.
- **Verworfen:** SQLite (Native-Dep + Schema-Migrations für v1-Iterationen
  zu viel Overhead), pure txt-Dateien (verlieren Metadaten wie
  Timestamps, Models, Tokens).
- **Trade-off:** Kein effizientes Querying (Full-Read für Stats). Für v1
  mit ~100–1000 Sessions akzeptabel. Bei Bedarf später Sidecar-Index
  oder Migration zu SQLite.
- **Reference:** SPEC-004 § Persistenz (in diesem Commit aktualisiert)

### Architektur-Verschlankung — C# weglassen, Tauri-Rust als Bridge

- **Decision:** C#-Backend-Layer wird ersatzlos gestrichen.
  Tauri-Rust übernimmt die Bridge-Logik (spawnt Copilot CLI als
  Subprozess, spricht JSON-RPC via Stdin/Stdout-Pipes) und nutzt
  dafür die Rust-Variante des Copilot SDK. **Kein Port wird für
  IPC geöffnet** — weder HTTP noch Named Pipe noch TCP.
- **Reason:** Eine Schicht weniger (2 statt 3 Prozesse), kein
  HTTP-localhost-Port für IPC nötig (nur OS-Pipes zwischen
  Tauri-Rust und CLI), ~5–15 MB Bundle-Ersparnis netto (kein
  .NET AOT-Runtime, aber Tauri-Rust exe wächst um Bridge-Code),
  eine Sprache weniger im Stack.
- **Verworfen:**
  - C# beibehalten (Status quo) — extra Schicht + Port + Bundle
    ohne klaren Mehrwert.
  - TS-SDK im Frontend mit „dummem Stream-Bridge" in Tauri-Rust
    — Rust-Bridge-Code wäre trotzdem nötig, TS-SDK-Logik nur
    Verdopplung der JSON-RPC-Schicht.
- **Trade-off:** Rust-Lernkurve (Martin 20+ Jahre .NET, Rust neu).
  Mitigation: Tauri ist Rust-nativ, große Community, viele
  Beispiele für genau dieses Subprozess-Pattern.
- **IPC-Verbindung:** Tauri-Rust ↔ CLI ausschließlich via
  `tokio::process::Command` + `Stdio::piped()` — kein HTTP, kein
  Named Pipe, kein TCP-Port.
- **Reference:** SPEC-001 (Architecture-Update), SPEC-004 (komplett
  neu als Rust-Bridge, File-Rename), SPEC-002 (Bundle ohne
  `backend/`-Ordner), SPEC-005 (IPC-Referenzen).

### Config-Storage: v1 Klartext, v3 DPAPI-TODO

- **Decision:** v1 speichert den `apiKey` in `config.json` als
  Klartext (UTF-8 String, kein Encoding). Eine echte
  Verschlüsselung (DPAPI auf Windows, Keychain auf macOS,
  Secret Service auf Linux) wird in v3 nachgerüstet.
- **Reason:** „Keep it simple" für v1 (Martins Direktive 2026-07-18):
  Single-User, Personal-Tool, keine Compliance-Anforderung für
  Encryption. Eine Windows-`dpapi.rs`-Implementation in v1 hätte
  FFI-Path-Mismatch-Risiken zwischen Crate-Versionen
  mitgebracht (siehe Commit-Versuche vor diesem hier). Klartext
  spart eine `windows`-crate-Dependency und reduziert die
  Build-Komplexität. Martins Direktive 2026-07-18 11:41:20:
  Code-Signing, Auto-Update und DPAPI zusammen auf v3 verschoben
  (Konsolidierung der Sicherheits-/Distribution-Features).
- **Verworfen für v1:**
  - DPAPI via `windows`-Crate — `CRYPT_DATA_BLOB`-Struct
    wandert zwischen Versionen (v0.58 fehlt am erwarteten Pfad),
    API-Signatur-Drift (BOOL → `Result<(), Error>`). v1-Timebudget
    zu kurz.
  - Master-Key aus Quellcode im Binary — Security-Theater.
- **Trade-off:** Bei physischem Zugriff auf den Rechner unter
  dem User-Account kann der apiKey aus `config.json` im Klartext
  gelesen werden. Akzeptabel für Single-User-Personal-Tool.
- **v3-Plan:** `keyring`-Crate für plattformübergreifende
  Secret-Storage (Windows-Credential-Manager → DPAPI-Wrapper,
  macOS-Keychain, Linux-Secret-Service). Migrations-Pfad ist
  in-place (v1-Klartext → v3-verschlüsselt beim ersten Start
  nach Update). DPAPI wird zusammen mit Code-Signing in v3
  eingeführt, da beide für sichere Distribution nötig sind
  (signed Binary + verschlüsselter Key als zusammengehörige
  Security-Features).
- **Reference:** SPEC-003 (`endpoint.apiKey` statt
  `apiKeyCipher`), Card #4 Implementation im Tauri-Rust-Bridge-
  Schritt. `src-tauri/src/config/dpapi.rs` enthält einen trivialen
  utf8-Passthrough, der in v3 durch eine echte Implementation
  ersetzt wird ohne API-Änderung für Caller.