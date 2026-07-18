import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./ConfigDialog.css";

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

interface ConfigTestResult {
  ok: boolean;
  models: string[];
  error: string | null;
}

type Tab = "connection" | "behavior" | "tools";
type Transport = "stdio" | "sse" | "http";

interface McpDraft {
  name: string;
  transport: Transport;
  command: string;
  url: string;
  enabled: boolean;
}

interface Props {
  onClose: () => void;
  initialConfig?: Config | null;
}

const EMPTY_CONFIG: Config = {
  endpoint: "",
  api_key: "",
  model: "",
  system_prompt: "",
  mcp_servers: [],
};

export default function ConfigDialog({ onClose, initialConfig }: Props) {
  const [tab, setTab] = useState<Tab>("connection");
  const [config, setConfig] = useState<Config>(
    initialConfig ?? EMPTY_CONFIG
  );
  const [testResult, setTestResult] = useState<ConfigTestResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [mcpDraft, setMcpDraft] = useState<McpDraft>({
    name: "",
    transport: "stdio",
    command: "",
    url: "",
    enabled: true,
  });

  useEffect(() => {
    (async () => {
      try {
        const existing = await invoke<Config | null>("config_get");
        if (existing) setConfig(existing);
      } catch {
        // Erstlauf — kein Config vorhanden, defaults bleiben
      }
    })();
  }, []);

  async function handleTest() {
    setTesting(true);
    setTestResult(null);
    setError(null);
    try {
      const result = await invoke<ConfigTestResult>("config_test", {
        endpoint: config.endpoint,
        apiKey: config.api_key,
      });
      setTestResult(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setTesting(false);
    }
  }

  async function handleSave() {
    setSaving(true);
    setError(null);
    try {
      await invoke("config_set", { config });
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  function addMcpServer() {
    if (!mcpDraft.name.trim()) return;
    const newServer: McpServer = {
      name: mcpDraft.name.trim(),
      transport: mcpDraft.transport,
      enabled: mcpDraft.enabled,
    };
    if (mcpDraft.transport === "stdio") {
      newServer.command = mcpDraft.command;
    } else {
      newServer.url = mcpDraft.url;
    }
    setConfig({
      ...config,
      mcp_servers: [...config.mcp_servers, newServer],
    });
    setMcpDraft({ name: "", transport: "stdio", command: "", url: "", enabled: true });
  }

  function removeMcpServer(index: number) {
    setConfig({
      ...config,
      mcp_servers: config.mcp_servers.filter((_, i) => i !== index),
    });
  }

  const canSave = Boolean(config.endpoint && config.api_key && config.model);

  return (
    <div className="config-dialog-overlay">
      <div className="config-dialog">
        <div className="dialog-header">
          <h2>BYOK-Setup</h2>
          <button
            className="close-btn"
            onClick={onClose}
            aria-label="Schließen"
            type="button"
          >
            ×
          </button>
        </div>

        <div className="dialog-tabs" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={tab === "connection"}
            className={tab === "connection" ? "tab active" : "tab"}
            onClick={() => setTab("connection")}
          >
            Connection
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={tab === "behavior"}
            className={tab === "behavior" ? "tab active" : "tab"}
            onClick={() => setTab("behavior")}
          >
            Behavior
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={tab === "tools"}
            className={tab === "tools" ? "tab active" : "tab"}
            onClick={() => setTab("tools")}
          >
            Tools (MCP)
          </button>
        </div>

        <div className="dialog-body">
          {error && <div className="error-banner" role="alert">{error}</div>}

          {tab === "connection" && (
            <div className="form-section">
              <label htmlFor="cfg-endpoint">Endpoint-URL</label>
              <input
                id="cfg-endpoint"
                type="text"
                placeholder="https://api.openai.com/v1"
                value={config.endpoint}
                onChange={(e) => setConfig({ ...config, endpoint: e.target.value })}
              />

              <label htmlFor="cfg-api-key">API-Key</label>
              <input
                id="cfg-api-key"
                type="password"
                placeholder="sk-…"
                value={config.api_key}
                onChange={(e) => setConfig({ ...config, api_key: e.target.value })}
              />

              <label htmlFor="cfg-model">Modell</label>
              <div className="row">
                <input
                  id="cfg-model"
                  type="text"
                  placeholder="gpt-4o"
                  value={config.model}
                  list="models-list"
                  onChange={(e) => setConfig({ ...config, model: e.target.value })}
                />
                <datalist id="models-list">
                  {testResult?.models.map((m) => (
                    <option key={m} value={m} />
                  ))}
                </datalist>
                <button
                  type="button"
                  className="secondary"
                  onClick={handleTest}
                  disabled={testing || !config.endpoint || !config.api_key}
                >
                  {testing ? "Testing…" : "Test Endpoint"}
                </button>
              </div>
              {testResult && (
                <div
                  className={
                    testResult.ok ? "test-ok" : "test-fail"
                  }
                  role="status"
                >
                  {testResult.ok
                    ? `✓ ${testResult.models.length} Models gefunden`
                    : `✗ ${testResult.error ?? "Test fehlgeschlagen"}`}
                </div>
              )}
            </div>
          )}

          {tab === "behavior" && (
            <div className="form-section">
              <label htmlFor="cfg-system-prompt">System Prompt</label>
              <textarea
                id="cfg-system-prompt"
                rows={12}
                placeholder="Leer = kein Custom-Prompt (Copilot CLI nutzt Default)."
                value={config.system_prompt ?? ""}
                onChange={(e) =>
                  setConfig({ ...config, system_prompt: e.target.value })
                }
              />
            </div>
          )}

          {tab === "tools" && (
            <div className="form-section">
              <h3>MCP-Server</h3>
              {config.mcp_servers.length === 0 ? (
                <p className="empty">Keine MCP-Server konfiguriert.</p>
              ) : (
                <ul className="mcp-list">
                  {config.mcp_servers.map((s, i) => (
                    <li key={`${s.name}-${i}`} className="mcp-row">
                      <span>
                        <strong>{s.name}</strong> ({s.transport})
                        {s.command ? ` — ${s.command}` : ""}
                        {s.url ? ` — ${s.url}` : ""}
                      </span>
                      <button
                        type="button"
                        className="danger small"
                        onClick={() => removeMcpServer(i)}
                      >
                        Entfernen
                      </button>
                    </li>
                  ))}
                </ul>
              )}

              <h4>Server hinzufügen</h4>
              <div className="mcp-add">
                <input
                  placeholder="name"
                  value={mcpDraft.name}
                  onChange={(e) =>
                    setMcpDraft({ ...mcpDraft, name: e.target.value })
                  }
                />
                <select
                  value={mcpDraft.transport}
                  onChange={(e) =>
                    setMcpDraft({
                      ...mcpDraft,
                      transport: e.target.value as Transport,
                    })
                  }
                >
                  <option value="stdio">stdio</option>
                  <option value="sse">sse</option>
                  <option value="http">http</option>
                </select>
                {mcpDraft.transport === "stdio" ? (
                  <input
                    placeholder="command (z. B. npx -y @mcp/server-filesystem)"
                    value={mcpDraft.command}
                    onChange={(e) =>
                      setMcpDraft({ ...mcpDraft, command: e.target.value })
                    }
                  />
                ) : (
                  <input
                    placeholder="url (z. B. http://localhost:3001)"
                    value={mcpDraft.url}
                    onChange={(e) =>
                      setMcpDraft({ ...mcpDraft, url: e.target.value })
                    }
                  />
                )}
                <button
                  type="button"
                  className="secondary"
                  onClick={addMcpServer}
                  disabled={!mcpDraft.name.trim()}
                >
                  Add
                </button>
              </div>
            </div>
          )}
        </div>

        <div className="dialog-footer">
          <button
            type="button"
            className="secondary"
            onClick={onClose}
            disabled={saving}
          >
            Abbrechen
          </button>
          <button
            type="button"
            className="primary"
            onClick={handleSave}
            disabled={saving || !canSave}
          >
            {saving ? "Speichern…" : "Speichern"}
          </button>
        </div>
      </div>
    </div>
  );
}
