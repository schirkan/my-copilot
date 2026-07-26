import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

/**
 * Vite-Plugin, das einen minimalen CopilotKit-/info-Endpoint
 * bereitstellt. Wir nutzen CopilotKit nur als UI-Provider (für
 * ggf. spätere Hooks wie useCopilotAction); der eigentliche
 * Chat läuft über Tauri-IPC `chat_send` → Node-Shim → BYOK.
 *
 * CopilotKit validiert beim Mount, dass eine der drei
 * Runtime-Quellen gesetzt ist (runtimeUrl, publicApiKey, …) und
 * feuert dann einen `GET /info`-Request. Damit dieser Request
 * im Dev-Modus nicht mit ECONNREFUSED abbricht (und das Console
 * mit `runtime_info_fetch_failed`-Errors flutet), antworten
 * wir hier mit einem No-op-Agent-Manifest.
 *
 * Nur aktiv im Dev-Server (`configureServer`); im Production-
 * Build irrelevant, weil der Bundle ohne dieses Plugin läuft.
 */
function copilotKitInfoStub(): Plugin {
  return {
    name: "copilotkit-info-stub",
    apply: "serve",
    configureServer(server) {
      server.middlewares.use("/copilotkit-runtime/info", (_req, res) => {
        res.setHeader("Content-Type", "application/json");
        // Minimale Manifest-Antwort: kein remote agent, leere
        // Capabilities. Der eigentliche Chat-Pfad geht durch
        // Tauri-IPC, nicht durch CopilotKit.
        res.end(
          JSON.stringify({
            version: "0.0.0-stub",
            agents: {},
            mode: "sse",
            telemetryDisabled: true,
            audioFileTranscriptionEnabled: false,
            threadEndpoints: undefined,
            suggestions: undefined,
            a2ui: undefined,
            a2uiEnabled: false,
            openGenerativeUIEnabled: false,
            intelligence: undefined,
            licenseStatus: undefined,
          }),
        );
      });
    },
  };
}

// https://vitejs.dev/config/
export default defineConfig({
  root: "src",
  plugins: [react(), copilotKitInfoStub()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    outDir: "../dist",
    target: "es2022",
    // Vite 8: Oxc-Minifier ist Default; "esbuild" wuerde esbuild als devDep erzwingen
    minify: true,
    sourcemap: false,
  },
});