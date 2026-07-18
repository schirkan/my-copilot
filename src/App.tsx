import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import ConfigDialog from "./ConfigDialog";
import ChatWindow from "./ChatWindow";
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
      // ignore
    }
  }

  function openSettings() {
    setShowDialog(true);
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
          {config ? `${config.endpoint} · ${config.model}` : "BYOK-Setup"}
        </p>
        <button
          type="button"
          className="settings-btn-top"
          onClick={openSettings}
        >
          Settings
        </button>
      </header>

      <ChatWindow />

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
