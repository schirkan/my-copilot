# SPEC-005 — Frontend (CopilotKit UI + Tauri IPC)

**Status:** Implementierung in Arbeit
**Datum:** 2026-07-17
**Bezug:** SPEC-001 § CopilotKit · SPEC-003 § Erstlauf-Flow ·
SPEC-004 § IPC-Methoden

## Verantwortlichkeit

Das React-Frontend ist die **User-Experience-Schicht** im
Tauri-WebView. Es:

1. Zeigt die **Chat-UI** (Nachrichten, Streaming-Response, Tool-Calls).
2. Stellt den **BYOK-Config-Screen** für Erstlauf und Updates bereit.
3. Verwaltet **UI-State** (offene Tabs, Dark/Light, Model-Auswahl).
4. Kommuniziert mit der **Tauri-Rust Bridge** via **Tauri-IPC**
   (`invoke` / `listen` / `emit`).
5. Nutzt **CopilotKit als UI-Schicht**, aber nicht als Runtime-Schicht;
  die Agent-Runtime sitzt in der Tauri-Rust-Bridge und wird über
  Tauri-Commands angesprochen.

## Tech-Stack (Frontend-only)

- **React 18+** mit TypeScript
- **CopilotKit UI-Komponenten / Hooks**, soweit sie keinen eigenen
  Runtime-Handshake erzwingen
- **Vite** als Build-Tool (Node.js nur zur Build-Zeit, nicht im
  Output)
- **Tailwind CSS** für Styling (kein Bundle-Bloat)
- **Zustand** für leichten globalen State (kein Redux nötig)
- **TanStack Query** für Tauri-Rust-Bridge-Calls (caching, retry)

## Komponenten-Hierarchie

```
<App>
  <ConfigGate>                  ← zeigt Config-Screen wenn keine config.json
    <EndpointForm />
  </ConfigGate>
  <ChatLayout>                  ← sonst
    <Sidebar>
      <SessionList />
      <NewChatButton />
      <SettingsButton />
    </Sidebar>
    <MainPanel>
      <CopilotKitUiShell>
        <ChatWindow>
          <MessageList>
            <UserMessage />
            <AssistantMessage />
          </MessageList>
          <ChatInput />
        </ChatWindow>
      </CopilotKitUiShell>
    </MainPanel>
  </ChatLayout>
</App>
```

## IPC-Anbindung (Tauri)

```typescript
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// Commands (Frontend → Backend)
export const api = {
  sendMessage: (msg: string) =>
    invoke("chat.send", { message: msg }),

  cancelMessage: (requestId: string) =>
    invoke("chat.cancel", { requestId }),

  getConfig: () =>
    invoke<Config>("config.get"),

  setConfig: (cfg: ConfigPayload) =>
    invoke("config.set", { config: cfg }),

  testEndpoint: (endpoint: string, apiKey: string) =>
    invoke<TestResult>("config.test", { endpoint, apiKey }),

  healthCheck: () =>
    invoke<HealthResult>("process.health"),

  restartProcess: () =>
    invoke("process.restart"),
};

// Events (Backend → Frontend)
export const events = {
};
```

## BYOK-Config-Screen (Erstlauf)

```tsx
function EndpointForm() {
  const [endpoint, setEndpoint] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("gpt-4o");
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<TestResult | null>(null);

  async function handleTest() {
    setTesting(true);
    const result = await api.testEndpoint(endpoint, apiKey);
    setTestResult(result);
    setTesting(false);
  }

  async function handleSave() {
    if (!testResult?.ok) return;
    await api.setConfig({ endpoint, apiKey, model });
    // App lädt neu → ChatLayout wird sichtbar
    location.reload();
  }

  return (
    <form>
      <h1>BYOK-Setup</h1>
      <select value={endpointType} onChange={...}>
        <option value="azure-openai">Azure OpenAI</option>
        <option value="openai">api.openai.com</option>
        <option value="openai-compatible">Self-hosted (LM Studio / vLLM)</option>
      </select>
      <input placeholder="https://..." value={endpoint} onChange={...} />
      <input type="password" placeholder="API-Key" value={apiKey} onChange={...} />
      <select value={model} onChange={...}>
        <option value="gpt-4o">gpt-4o</option>
        <option value="gpt-4o-mini">gpt-4o-mini</option>
        <option value="claude-3.5-sonnet">Claude 3.5 Sonnet (via OpenRouter)</option>
      </select>
      <button onClick={handleTest} disabled={testing}>
        {testing ? "Testing..." : "Test Endpoint"}
      </button>
      {testResult && (
        <div className={testResult.ok ? "ok" : "error"}>
          {testResult.ok
            ? `✓ ${testResult.models.length} Models gefunden`
            : `✗ ${testResult.error}`}
        </div>
      )}
      <button onClick={handleSave} disabled={!testResult?.ok}>
        Speichern & Starten
      </button>
    </form>
  );
}
```

## Chat-Integration

```tsx
function ChatWindow() {
  const [messages, setMessages] = useState([]);
  const [isLoading, setIsLoading] = useState(false);

  return (
    <div className="chat-window">
      {messages.map((m) => (
        <MessageBubble key={m.id} message={m} />
      ))}
      {isLoading && <TypingIndicator />}
      <ChatInput
        onSubmit={async (text) => {
          setIsLoading(true);
          const reply = await api.sendMessage(text);
          setIsLoading(false);
          // append reply to local state
        }}
      />
    </div>
  );
}
```

**Wichtig:** CopilotKit darf in diesem Projekt nur als visuelle/UI-
Abstraktion verwendet werden. Es darf keinen eigenen Runtime-Connect
auslösen. Die Source of Truth für Nachrichtenfluss, Sessions und BYOK
liegt bei Tauri-IPC + Rust-SDK.

## State Management

- **Lokal**: React `useState` für Form-Inputs, modals, etc.
- **Global**: Zustand-Store für `currentSessionId`, `theme`,
  `userSettings`
- **Server-State**: TanStack Query optional für Tauri-Rust-Bridge-Calls
  (caching, retry)

## Build-Setup (Vite)

```json
// package.json
{
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "react": "^18.3.0",
    "react-dom": "^18.3.0",
    "@copilotkit/react-core": "^latest",
    "@copilotkit/react-ui": "^latest",
    "@tauri-apps/api": "^2.0.0",
    "zustand": "^4.5.0",
    "@tanstack/react-query": "^5.0.0"
  }
}
```

```typescript
// vite.config.ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "../dist",
    target: "es2022",
    minify: "esbuild",
  },
});
```

## Offene Punkte

- **Session-Liste Sidebar**: lokale JSONL-Reads (alle Session-Files
  in `data/sessions/` scannen, Metadaten aus erster Zeile) oder
  in-memory-Index mit Sidecar-Cache-File?
- **Dark/Light-Mode**: System-Präferenz folgen oder User-Settings?
- **Markdown-Rendering** für Assistant-Responses: react-markdown?
- **Tool-/Permission-UI**: später über SDK-Session-Events ergänzen?
- **i18n**: Deutsch + Englisch oder nur Deutsch (Martins Use-Case)?

## Quellen

- CopilotKit Docs — copilotkit.ai
- Tauri 2 IPC — tauri.app
- React 18 Docs — react.dev
- Vite — vitejs.dev