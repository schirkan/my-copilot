import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ConfigDialog from "./ConfigDialog";
import "./App.css";

interface McpServer {
  name: string;
  transport: string;
  command?: string;
  url?: string;
  env?: Record<string, string>;
  enabled: boolean;
}

interface Config {
  endpoint: string;
  api_key: string;
  model: string;
  system_prompt?: string;
  mcp_servers: McpServer[];
}

function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [showDialog, setShowDialog] = useState(false);
  const [loading, setLoading] = useState(true);
  const [count, setCount] = useState(0);

  useEffect(() => {
    (async () => {
      try {
        const existing = await invoke<Config | null>("config_get");
        if (existing) {
          setConfig(existing);
        } else {
          setShowDialog(true);
        }
      } catch {
        // Backend nicht erreichbar oder kein Config — Dialog zeigen
        setShowDialog(true);
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  async function handleDialogClose() {
    setShowDialog(false);
    try {
      const updated = await invoke<Config | null>("config_get");
      if (updated) setConfig(updated);
    } catch {
      // ignore — bestehende Config behalten
    }
  }

  if (loading) {
    return (
      <main className="container">
        <p>Lade Konfiguration…</p>
      </main>
    );
  }

  if (showDialog) {
    return (
      <main className="container">
        <ConfigDialog onClose={handleDialogClose} initialConfig={config} />
      </main>
    );
  }

  return (
    <main className="container">
      <header className="hero">
        <h1>My Copilot</h1>
        <p className="subtitle">
          Portable Windows 11 desktop app for AI agent workflows
        </p>
        {config && (
          <p className="version">
            {config.endpoint} · {config.model}
          </p>
        )}
      </header>

      <section className="card">
        <h2>Tauri 2 + React 18 + TypeScript 5</h2>
        <p>Hello, World! Skeleton is alive.</p>
        <button onClick={() => setCount((c) => c + 1)}>
          Clicks: {count}
        </button>
      </section>

      <section className="card stack">
        <h3>Tech-Stack</h3>
        <ul>
          <li>App-Shell + Bridge: Tauri 2 (Rust)</li>
          <li>Subprozess: Node.js v22+ + GitHub Copilot CLI</li>
          <li>Frontend: CopilotKit React + Vite</li>
          <li>LLM-Provider: OpenAI-kompatibel (BYOK)</li>
        </ul>
      </section>

      <section className="card">
        <button
          type="button"
          className="settings-btn"
          onClick={() => setShowDialog(true)}
        >
          Settings
        </button>
      </section>

      <footer className="footer">
        <small>
          Architektur: SPEC-001 · Bundle: SPEC-002 · Persistence: JSONL
          (siehe DECISIONS.md)
        </small>
      </footer>
    </main>
  );
}

export default App;
