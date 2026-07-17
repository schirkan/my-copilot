# SPEC-005 — Frontend (CopilotKit React)

**Status:** Planungs-Phase
**Datum:** 2026-07-17
**Bezug:** SPEC-001 § CopilotKit · SPEC-003 § Erstlauf-Flow ·
SPEC-004 § IPC-Methoden

## Verantwortlichkeit

Das React-Frontend ist die **User-Experience-Schicht** im
Tauri-WebView. Es:

1. Zeigt die **Chat-UI** (Nachrichten, Streaming-Response, Tool-Calls).
2. Stellt den **BYOK-Config-Screen** für Erstlauf und Updates bereit.
3. Verwaltet **UI-State** (offene Tabs, Dark/Light, Model-Auswahl).
4. Kommuniziert mit dem C#-Backend via **Tauri-IPC**
   (`invoke` / `emit`).
5. Nutzt **CopilotKit-Komponenten** für AI-Chat (Generative UI,
   Tool-UI).

## Tech-Stack (Frontend-only)

- **React 18+** mit TypeScript
- **CopilotKit** (`@copilotkit/react-core`, `@copilotkit/react-ui`)
- **Vite** als Build-Tool (Node.js nur zur Build-Zeit, nicht im
  Output)
- **Tailwind CSS** für Styling (kein Bundle-Bloat)
- **Zustand** für leichten globalen State (kein Redux nötig)
- **TanStack Query** für C#-Backend-Calls (caching, retry)

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
      <CopilotKit runtime={...}>
        <ChatWindow>
          <MessageList>
            <UserMessage />
            <AssistantMessage streaming />
            <ToolCallBadge />
          </MessageList>
          <ChatInput />
        </ChatWindow>
      </CopilotKit>
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
  onChunk: (cb: (data: { requestId: string; text: string }) => void) =>
    listen("chat.chunk", (e) => cb(e.payload)),

  onDone: (cb: (data: { requestId: string; usage: TokenUsage }) => void) =>
    listen("chat.done", (e) => cb(e.payload)),

  onError: (cb: (data: { requestId: string; error: string }) => void) =>
    listen("chat.error", (e) => cb(e.payload)),
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

## CopilotKit-Integration

```tsx
import { CopilotKit } from "@copilotkit/react-core";
import { useCopilotChat } from "@copilotkit/react-core";

function ChatWindow() {
  const { messages, sendMessage, isLoading } = useCopilotChat();

  return (
    <CopilotKit runtime={tauriRuntime}>
      <div className="chat-window">
        {messages.map((m) => (
          <MessageBubble key={m.id} message={m} />
        ))}
        {isLoading && <TypingIndicator />}
        <ChatInput onSubmit={sendMessage} />
      </div>
    </CopilotKit>
  );
}

// Runtime-Wrapper: Tauri → C# → Copilot SDK
const tauriRuntime = {
  // CopilotKit's RemoteAction-Adapter, der via Tauri-IPC ans C#-Backend geht
  async chat({ messages, tools }) {
    // ... mapped zu api.sendMessage / events.onChunk
  },
};
```

## Tool-UI (Generative UI)

CopilotKit erlaubt deklarative Tool-UI:

```tsx
import { useCopilotAction } from "@copilotkit/react-core";

function FileReadTool() {
  useCopilotAction({
    name: "read_file",
    description: "Reads a file from the local filesystem",
    parameters: [
      { name: "path", type: "string", description: "Absolute file path" },
    ],
    render: ({ args, status }) => (
      <div className="tool-card">
        📄 Reading <code>{args.path}</code>
        {status === "complete" && " ✓"}
      </div>
    ),
  });
}
```

## State Management

- **Lokal**: React `useState` für Form-Inputs, modals, etc.
- **Global**: Zustand-Store für `currentSessionId`, `theme`,
  `userSettings`
- **Server-State**: TanStack Query für C#-Backend-Calls (caching,
  retry)

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
- **Tool-UI-Bibliothek**: shadcn/ui als Basis?
- **i18n**: Deutsch + Englisch oder nur Deutsch (Martins Use-Case)?

## Quellen

- CopilotKit Docs — copilotkit.ai
- Tauri 2 IPC — tauri.app
- React 18 Docs — react.dev
- Vite — vitejs.dev