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

### Code-Signing

- **Decision:** **Ohne** Code-Signing für v1.
- **Reason:** SmartScreen-Warning beim Erststart ist lästig, aber für
  Personal-Tool akzeptabel. EV-Code-Signing-Zertifikat (~300 €/Jahr)
  lohnt erst bei öffentlichem Release mit Fremd-Usern.
- **Verworfen:** Self-Signed (Vertrauens-Chaos), EV-Cert (Kosten/Nutzen
  für v1 zu hoch).
- **Reference:** SPEC-002 § Offene Punkte

### Distribution-Channel

- **Decision:** GitHub Releases für v1.
- **Reason:** Kostenlos, ZIP-Download, passt zum Repo, keine eigene
  Infrastruktur nötig.
- **Verworfen:** Eigener Server / CDN — Overhead für v1, frühestens v2.
- **Reference:** SPEC-002 § Distribution