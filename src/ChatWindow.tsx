import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
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
  /** Request-ID, mit der diese Message erzeugt wurde. Wird fuer
   *  Streaming (`chat_chunk`-Events) und Cancel verwendet. */
  request_id?: string;
}

interface PersistedMessage {
  id: string;
  role: string;
  content: string;
}

// -----------------------------------------------------------------------------
// Streaming-Event-Payloads (siehe src-tauri/src/commands/chat.rs)
// -----------------------------------------------------------------------------

interface ChatChunkPayload {
  request_id: string;
  delta: string;
  accumulated: string;
}

interface ChatDonePayload {
  request_id: string;
  content: string;
}

interface ChatErrorPayload {
  request_id: string;
  error: string;
}

// -----------------------------------------------------------------------------
// Response auf `chat_send`
// -----------------------------------------------------------------------------

interface ChatSendResponse {
  /** Stabile Session-ID (mehrere Messages teilen dieselbe ID). Wird
   *  vom Client gecached und beim naechsten `chat_send` zurueckgegeben,
   *  damit User + Assistant beider Messages im selben JSONL landen. */
  session_id: string;
  /** Transient Korrelations-ID fuer die Streaming-Events dieses Calls. */
  request_id: string;
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

function sanitizeAssistantContent(text: string): string {
  return text
    .replace(/<think>[\s\S]*?<\/think>/gi, "")
    .replace(/^```markdown\s*/i, "```")
    .trim();
}

function makeId(): string {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    return crypto.randomUUID();
  }
  return `msg-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

/**
 * Lokale Chat-UI fuer My Copilot mit Echtzeit-Streaming.
 *
 * Architektur (siehe SPEC-001 + SPEC-005 + commands/chat.rs):
 *  - Frontend ruft `chat_send` auf, das sofort die `request_id`
 *    zurueckgibt.
 *  - Tauri-Events `chat_chunk` / `chat_done` / `chat_error` liefern
 *    die Assistant-Response in Echtzeit (Token-fuer-Token).
 *  - User-Message wird VOR dem Stream persistiert, Assistant-Message
 *    NACH `chat_done` (oder gar nicht bei `chat_error`).
 *  - Cancel: `chat_cancel(request_id)` -> session.abort() -> loop
 *    sieht `session.idle`/`session.error` und emittiert `chat_done`/
 *    `chat_error` (mit partial content).
 *  - CopilotKit ist aktuell bewusst NICHT im Render-Tree (siehe
 *    vorherige Commit-Historie).
 */
function ChatInner() {
  const [input, setInput] = useState("");
  const [sessions, setSessions] = useState<SessionMeta[]>([]);
  const [currentSessionId, setCurrentSessionId] = useState<string | null>(
    null,
  );
  const [localMessages, setLocalMessages] = useState<ChatMessage[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [activeRequestId, setActiveRequestId] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  // ---------------------------------------------------------------------------
  // Streaming-Event-Subscriptions
  // ---------------------------------------------------------------------------
  //
  // Wir registrieren drei Listener, die per `request_id` die richtige
  // Assistant-Bubble finden und inkrementell aktualisieren.
  useEffect(() => {
    const unlistens: UnlistenFn[] = [];

    void (async () => {
      // `chat_chunk` -- einzelnes Text-Delta. Wir nutzen `accumulated`
      // direkt vom Backend (kein eigenes append noetig), damit
      // Race-Conditions bei schnellen Chunks ausgeschlossen sind.
      // (Tauri 2 erlaubt in Event-Namen keine Punkte, daher snake_case.)
      unlistens.push(
        await listen<ChatChunkPayload>("chat_chunk", (event) => {
          const { request_id, accumulated } = event.payload;
          setLocalMessages((prev) =>
            prev.map((m) =>
              m.request_id === request_id
                ? { ...m, content: sanitizeAssistantContent(accumulated) }
                : m,
            ),
          );
        }),
      );

      // `chat_done` -- Antwort fertig. Content ist vollstaendig,
      // loading-State wird gecleart.
      unlistens.push(
        await listen<ChatDonePayload>("chat_done", (event) => {
          const { request_id, content } = event.payload;
          setLocalMessages((prev) =>
            prev.map((m) =>
              m.request_id === request_id
                ? { ...m, content: sanitizeAssistantContent(content) }
                : m,
            ),
          );
          setIsLoading(false);
          setActiveRequestId(null);
          // History refresh (JSONL-Persistenz ist in Rust abgeschlossen)
          window.setTimeout(() => {
            void (async () => {
              try {
                const list = await invoke<SessionMeta[]>(
                  "history_list_sessions",
                );
                setSessions(list);
              } catch (e) {
                // eslint-disable-next-line no-console
                console.error("history_list_sessions failed:", e);
              }
            })();
          }, 300);
        }),
      );

      // `chat_error` -- Fehler beim Stream. Content bleibt auf dem
      // letzten Chunk stehen, error-Marker wird vorangestellt.
      unlistens.push(
        await listen<ChatErrorPayload>("chat_error", (event) => {
          const { request_id, error } = event.payload;
          setLocalMessages((prev) =>
            prev.map((m) =>
              m.request_id === request_id
                ? {
                  ...m,
                  content: `[Fehler: ${error}]`,
                }
                : m,
            ),
          );
          setIsLoading(false);
          setActiveRequestId(null);
        }),
      );
    })();

    return () => {
      for (const un of unlistens) {
        un();
      }
    };
  }, []);

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

    // Wenn der User gerade eine Session geladen hat (oder schon
    // Nachrichten in dieser View hat), verwende deren ID. Sonst
    // ueberlassen wir die Vergabe Rust (UUID v4).
    const sessionIdToUse = currentSessionId;

    setInput("");
    setIsLoading(true);

    // Wir zeigen User-Message und leere Assistant-Bubble ERST NACH
    // der Server-Response an -- so koennen wir direkt die
    // Server-Request-ID als Marker verwenden. Das verhindert, dass
    // die ersten `chat_chunk`-Events vor dem localen Marker-Override
    // ankommen und in der falschen Bubble landen.
    void invoke<ChatSendResponse>("chat_send", {
      message: trimmed,
      sessionId: sessionIdToUse,
    })
      .then((resp) => {
        // Falls wir gerade eine neue Session gestartet haben, ist die
        // vom Server vergebene session_id jetzt unsere currentSessionId.
        if (!currentSessionId) {
          setCurrentSessionId(resp.session_id);
        }
        // Optimistic UI: User-Message + leere Assistant-Bubble, die
        // die Server-Request-ID als Marker traegt.
        setLocalMessages((prev) => [
          ...prev,
          { id: makeId(), role: "user", content: trimmed },
          {
            id: makeId(),
            role: "assistant",
            content: "",
            request_id: resp.request_id,
          },
        ]);
        setActiveRequestId(resp.request_id);
      })
      .catch((err: unknown) => {
        const msg = err instanceof Error ? err.message : String(err);
        // Bei Fehler die User-Message NICHT in den Chat rendern (sonst
        // steht sie ewig ohne Antwort da). Wir zeigen stattdessen
        // einen Fehler-Bubble statt der User-Message.
        setLocalMessages((prev) => [
          ...prev,
          { id: makeId(), role: "user", content: trimmed },
          {
            id: makeId(),
            role: "assistant",
            content: `[Fehler: ${msg}]`,
          },
        ]);
        setIsLoading(false);
        setActiveRequestId(null);
        // eslint-disable-next-line no-console
        console.error("chat_send failed:", err);
      });
  }

  async function handleCancel() {
    if (!activeRequestId) return;
    try {
      await invoke("chat_cancel", { requestId: activeRequestId });
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error("chat_cancel failed:", e);
    }
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
          content:
            msg.role === "assistant"
              ? sanitizeAssistantContent(msg.content)
              : msg.content,
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
                {m.role === "assistant" ? (
                  extractText(m.content) ? (
                    <ReactMarkdown remarkPlugins={[remarkGfm]}>
                      {extractText(m.content)}
                    </ReactMarkdown>
                  ) : isLoading && m.request_id === activeRequestId ? (
                    <span className="typing-indicator">…</span>
                  ) : null
                ) : (
                  extractText(m.content)
                )}
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
          <div className="input-buttons">
            {isLoading && (
              <button
                type="button"
                className="cancel-btn"
                onClick={handleCancel}
                aria-label="Cancel current request"
              >
                Abbrechen
              </button>
            )}
            <button
              type="button"
              className="primary"
              onClick={handleSend}
              disabled={isLoading || !input.trim()}
            >
              {isLoading ? "…" : "Senden"}
            </button>
          </div>
        </div>
      </main>
    </div>
  );
}

export default function ChatWindow() {
  return <ChatInner />;
}
