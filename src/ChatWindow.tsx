import { useEffect, useRef, useState } from "react";
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
 *  - CopilotKit wird genutzt, weil es UI-Komponenten + Hooks für
 *    Chat-Verhalten bereitstellt (message state, append, reload,
 *    stop). Der eingebaute Remote-Runtime-Pfad (`runtimeUrl`)
 *    wird NICHT verwendet — die Validierung von CopilotKit erwartet
 *    aber `runtimeUrl` ODER `publicApiKey`/etc., deswegen übergeben
 *    wir einen Platzhalter. Die eigentliche Chat-Pipeline läuft
 *    über `onSubmitMessage` + den Tauri-IPC.
 *  - `appendMessage(msg, { followUp: false })` verhindert, dass
 *    CopilotKit nach dem Append versucht, den Remote-Agent zu
 *    kontaktieren.
 */
function ChatInner() {
  const { appendMessage } = useCopilotChat({
    onSubmitMessage: async (text: string) => {
      const assistantId = makeId();
      // Optimistic UI: leere Assistant-Message anzeigen
      setLocalMessages((prev) => [
        ...prev,
        {
          id: assistantId,
          role: "assistant",
          content: "",
        },
      ]);
      setIsLoading(true);
      try {
        const reply = await invoke<string>("chat_send", { message: text });
        setLocalMessages((prev) =>
          prev.map((m) =>
            m.id === assistantId ? { ...m, content: reply } : m,
          ),
        );
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setLocalMessages((prev) =>
          prev.map((m) =>
            m.id === assistantId
              ? {
                  ...m,
                  content: `[Fehler: ${msg}]`,
                }
              : m,
          ),
        );
        // eslint-disable-next-line no-console
        console.error("chat_send failed:", err);
      } finally {
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
            } catch (e) {
              // eslint-disable-next-line no-console
              console.error("history_list_sessions failed:", e);
            }
          })();
        }, 500);
      }
    },
  });

  // useCopilotChat (v1.63 headless hook) liefert KEIN input/setInput.
  // Diese müssen lokal verwaltet werden (siehe SPEC-005 § ChatInput).
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

    // followUp:false verhindert, dass CopilotKit versucht, den
    // Remote-Runtime-Agent zu kontaktieren — wir machen das selbst
    // in onSubmitMessage.
    void appendMessage(
      { role: "user", content: trimmed },
      { followUp: false },
    );
    setInput("");
  }

  async function handleLoadSession(sessionId: string) {
    setCurrentSessionId(sessionId);
    // v1: History-View im Sidebar (in v2 als separate Messages-Ansicht)
    try {
      const msgs = await invoke<unknown[]>("history_load_session", {
        sessionId,
      });
      // eslint-disable-next-line no-console
      console.info(
        `Loaded ${msgs.length} messages from session ${sessionId} (v1: log only)`,
      );
    } catch (e) {
      // eslint-disable-next-line no-console
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
  // runtimeUrl ist ein Platzhalter — CopilotKit validiert nur, dass
  // EINES von runtimeUrl/publicApiKey/publicLicenseKey gesetzt ist.
  // Wir rufen den Remote-Runtime-Pfad nie auf, weil wir in
  // ChatInner mit `appendMessage(msg, { followUp: false })` arbeiten
  // und den eigentlichen Chat über Tauri-IPC `chat_send` abwickeln.
  const runtimeUrl = "http://localhost/copilotkit-runtime";
  return (
    <CopilotKit runtimeUrl={runtimeUrl}>
      <ChatInner />
    </CopilotKit>
  );
}
