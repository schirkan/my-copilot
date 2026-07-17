import { useState } from "react";
import "./App.css";

function App() {
  const [count, setCount] = useState(0);

  return (
    <main className="container">
      <header className="hero">
        <h1>My Copilot</h1>
        <p className="subtitle">
          Portable Windows 11 desktop app for AI agent workflows
        </p>
        <p className="version">v0.1.0 · Skeleton (M1)</p>
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