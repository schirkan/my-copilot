import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { CopilotKit, useCopilotChat } from "@copilotkit/react-core";
import "./ChatWindow.css";

interface SessionMeta {
  session_id: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  model: string;
  title: string;
}

interface ChatContentPart {
  type?: string;
  text?: string;
}

interface ChatMessage {
  id: string;
  role: string;
  content: string | ChatContentPart[];
}

function extractText(content: string | ChatContentPart[] | undefined): string {
  if (typeof content === "string") {
    return content;
  }
  if (Array.isArray(content)) {
    return content
      .map((p) => (typeof p?.text === "string" ? p.text : ""))
      .filter((s) => s.length > 0)
      .join("\n");
  }
  return "";
}

/**
 * Custom CopilotKit-Runtime, das die Tauri-IPC `chat_send` aufruft.
 * Damit läuft die ganze Chat-Pipeline lokal (kein externes HTTP-Backend).
 */
function createTauriRuntime() {
  return {
    chat: {
      async chatCompletion({
        messages,
      }: {
        messages: ChatMessage[];
      }): Promise<ChatMessage> {
        const lastMsg = messages[messages.length - 1];
        const content = extractText(lastMsg?.content);

        const response = await invoke<string>("chat_send", {
          message: content,
        });

        return {
          id:
            typeof crypto !== "undefined" && "randomUUID" in crypto
              ? crypto.randomUUID()
              : `msg-${Date.now()}`,
          role: "assistant",
          content: response,
        };
      },
    },
  };
}

function ChatInner() {
  const { messages, input, setInput, appendMessage, isLoading } =
    useCopilotChat();
  const [sessions, setSessions] = useState<SessionMeta[]>([]);
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(
    null,
  );
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  // Session-Liste beim Mount laden
  useEffect(() => {
    void (async () => {
      try {
        const list = await invoke<SessionMeta[]>("history_list_sessions");
        setSessions(list);
      } catch (e) {
        console.error("history_list_sessions failed:", e);
      }
    })();
  }, []);

  // Auto-Scroll zur neuesten Message
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  function handleSend() {
    const trimmed = input.trim();
    if (!trimmed || isLoading) return;
    void appendMessage({ role: "user", content: trimmed });
    setInput("");
    // Session-Liste refreshen nach kurzer Verzögerung
    window.setTimeout(() => {
      void (async () => {
        try {
          const list = await invoke<SessionMeta[]>(
            "history_list_sessions",
          );
          setSessions(list);
        } catch (e) {
          console.error("history_list_sessions failed:", e);
        }
      })();
    }, 1000);
  }

  async function handleLoadSession(sessionId: string) {
    setCurrentSessionId(sessionId);
    // v1: History-View im Sidebar (in v2 als separate Messages-Ansicht)
    try {
      const msgs = await invoke<unknown[]>("history_load_session", {
        sessionId,
      });
      console.info(
        `Loaded ${msgs.length} messages from session ${sessionId} (v1: log only)`,
      );
    } catch (e) {
      console.error("history_load_session failed:", e);
    }
  }

  return (
    <div className="chat-window">
      <aside className="chat-sidebar">
        <h3>Sessions</h3>
        {sessions.length === 0 ? (
          <p className="sidebar-empty">Noch keine Sessions.</p>
        ) : (
          <ul>
            {sessions.map((s) => (
              <li
                key={s.session_id}
                className={
                  s.session_id === currentSessionId ? "active" : undefined
                }
                onClick={() => {
                  void handleLoadSession(s.session_id);
                }}
              >
                <div className="session-title">{s.title}</div>
                <div className="session-meta">
                  {s.message_count} msgs · {s.model}
                </div>
              </li>
            ))}
          </ul>
        )}
      </aside>

      <main className="chat-main">
        <div className="message-list">
          {messages.length === 0 && (
            <div className="empty">
              Neue Session — frag mich etwas.
            </div>
          )}
          {messages.map((m: ChatMessage) => (
            <div
              key={m.id}
              className={`message ${m.role === "user" ? "user" : "assistant"}`}
            >
              <div className="message-role">
                {m.role === "user" ? "Du" : "Copilot"}
              </div>
              <div className="message-content">
                {extractText(m.content)}
              </div>
            </div>
          ))}
          {isLoading && (
            <div className="message loading">… denke nach …</div>
          )}
          <div ref={messagesEndRef} />
        </div>

        <div className="input-area">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder="Frage eingeben… (Enter sendet, Shift+Enter newline)"
            rows={3}
            disabled={isLoading}
          />
          <button
            type="button"
            className="primary"
            onClick={handleSend}
            disabled={isLoading || !input.trim()}
          >
            {isLoading ? "…" : "Senden"}
          </button>
        </div>
      </main>
    </div>
  );
}

export default function ChatWindow() {
  const runtime = useMemo(() => createTauriRuntime(), []);
  return (
    <CopilotKit runtime={runtime}>
      <ChatInner />
    </CopilotKit>
  );
}
