import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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

interface PersistedMessage {
  id: string;
  role: string;
  content: string;
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

function makeId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

/**
 * Lokale Chat-UI für My Copilot.
 *
 * Architektur (siehe SPEC-001 + SPEC-005):
 *  - Tauri-IPC-Command `chat_send` ruft den Copilot-SDK-Subprozess
 *    über die Tauri-Rust-Bridge auf (kein HTTP, kein externes Backend).
 *  - CopilotKit ist aktuell bewusst NICHT im Render-Tree. Die lokale
 *    Chat-Logik spricht direkt Tauri-IPC `chat_send`; ein CopilotKit-
 *    Provider würde in v1.63.x sonst automatisch eine Runtime-Info
 *    gegen die CopilotKit-Cloud auflösen.
 *  - Frühere Versuche mit `useCopilotChat({ onSubmitMessage })` oder
 *    `appendMessage(plainObject)` crashen, weil der Hook in v1.63.1
 *    CopilotKit-Message-Instanzen (mit `isResultMessage()`) erwartet
 *    bzw. intern GraphQL-Remote-Runtime ansprechen würde. Daher
 *    managen wir Messages und Submit-Pfad jetzt komplett lokal.
 */
function ChatInner() {
  const [input, setInput] = useState("");
  const [sessions, setSessions] = useState<SessionMeta[]>([]);
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(
    null,
  );
  const [localMessages, setLocalMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  // Session-Liste beim Mount laden
  useEffect(() => {
    void (async () => {
      try {
        const list = await invoke<SessionMeta[]>("history_list_sessions");
        setSessions(list);
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error("history_list_sessions failed:", e);
      }
    })();
  }, []);

  // Auto-Scroll zur neuesten Message
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [localMessages]);

  function handleSend() {
    const trimmed = input.trim();
    if (!trimmed || isLoading) return;

    // User-Message lokal anzeigen
    setLocalMessages((prev) => [
      ...prev,
      { id: makeId(), role: "user", content: trimmed },
    ]);
    setInput("");

    // Optimistic UI: leere Assistant-Bubble sofort rendern, damit
    // der Lade-Zustand visuell klar ist.
    const assistantId = makeId();
    setLocalMessages((prev) => [
      ...prev,
      { id: assistantId, role: "assistant", content: "" },
    ]);
    setIsLoading(true);

    void invoke<string>("chat_send", { message: trimmed })
      .then((reply) => {
        setLocalMessages((prev) =>
          prev.map((m) =>
            m.id === assistantId ? { ...m, content: reply } : m,
          ),
        );
      })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        setLocalMessages((prev) =>
          prev.map((m) =>
            m.id === assistantId
              ? { ...m, content: `[Fehler: ${msg}]` }
              : m,
          ),
        );
        // eslint-disable-next-line no-console
        console.error("chat_send failed:", err);
      })
      .finally(() => {
        setIsLoading(false);
        // Session-Liste refreshen (JSONL-Persistenz ist in Rust
        // abgeschlossen, sobald chat_send zurückkehrt).
        window.setTimeout(() => {
          void (async () => {
            try {
              const list = await invoke<SessionMeta[]>(
                "history_list_sessions",
              );
              setSessions(list);
              if (list.length > 0) {
                setCurrentSessionId(list[0].session_id);
              }
            } catch (e) {
              // eslint-disable-next-line no-console
              console.error("history_list_sessions failed:", e);
            }
          })();
        }, 500);
      });
  }

  async function handleLoadSession(sessionId: string) {
    setCurrentSessionId(sessionId);
    try {
      const msgs = await invoke<PersistedMessage[]>("history_load_session", {
        sessionId,
      });
      setLocalMessages(
        msgs.map((msg) => ({
          id: msg.id,
          role: msg.role,
          content: msg.content,
        })),
      );
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("history_load_session failed:", e);
    }
  }

  async function handleDeleteSession(
    event: React.MouseEvent<HTMLButtonElement>,
    sessionId: string,
  ) {
    event.stopPropagation();
    try {
      await invoke("history_delete_session", { sessionId });
      setSessions((prev) => prev.filter((s) => s.session_id !== sessionId));
      if (currentSessionId === sessionId) {
        setCurrentSessionId(null);
        setLocalMessages([]);
      }
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("history_delete_session failed:", e);
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
                <div className="session-row">
                  <div className="session-text">
                    <div className="session-title">{s.title}</div>
                    <div className="session-meta">
                      {s.message_count} msgs · {s.model}
                    </div>
                  </div>
                  <button
                    type="button"
                    className="session-delete"
                    aria-label={`Delete session ${s.title}`}
                    onClick={(event) => {
                      void handleDeleteSession(event, s.session_id);
                    }}
                  >
                    ×
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </aside>

      <main className="chat-main">
        <div className="message-list">
          {localMessages.length === 0 && (
            <div className="empty">
              Neue Session — frag mich etwas.
            </div>
          )}
          {localMessages.map((m) => (
            <div
              key={m.id}
              className={`message ${m.role === "user" ? "user" : "assistant"}`}
            >
              <div className="message-role">
                {m.role === "user" ? "Du" : "Copilot"}
              </div>
              <div className="message-content">
                {extractText(m.content) ||
                  (m.role === "assistant" && isLoading ? "…" : "")}
              </div>
            </div>
          ))}
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
  return <ChatInner />;
}
