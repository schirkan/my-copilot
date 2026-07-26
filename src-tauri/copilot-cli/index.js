#!/usr/bin/env node
/**
 * My Copilot — Copilot CLI Shim (BYOK)
 *
 * Spricht JSON-RPC 2.0 mit der Tauri-Rust-Bridge (siehe SPEC-004).
 * Liest eine JSON-RPC-Request-Zeile von stdin, proxied sie an den
 * konfigurierten OpenAI-kompatiblen Endpoint (streaming), und
 * schreibt JSON-RPC-Notifications (`{"method":"chat.chunk",...}`)
 * pro Token zurück nach stdout.
 *
 * Aufbau (siehe DECISIONS.md § Architektur-Verschlankung):
 *  - Rust-Bridge → Stdin (1 Zeile = 1 Request)
 *  - Diese Datei → OpenAI-kompatibles HTTP-Streaming
 *  - Diese Datei → Stdout (1 Zeile = 1 Notification ODER Response)
 *  - stderr → wird von der Rust-Bridge in den Log geschrieben
 *
 * Kein externer Dependency-Overhead — nur Node.js-Builtins.
 *
 * Usage (von Rust aus):
 *   node <this-file>
 *
 * Erwartete stdin-Zeile (eine JSON-RPC-Request):
 *   {
 *     "jsonrpc": "2.0",
 *     "id": 1,
 *     "method": "chat",
 *     "params": {
 *       "message": "Hallo Welt",
 *       "model": "MiniMax-M3",
 *       "api_key": "...",
 *       "endpoint": "https://api.minimax.io/v1",
 *       "system_prompt": "Du bist ein hilfreicher Assistent.",
 *       "mcp_servers": []
 *     }
 *   }
 *
 * Geschriebene stdout-Zeilen:
 *   - {"jsonrpc":"2.0","id":1,"result":{"ok":true}}      ← Acknowledge
 *   - {"jsonrpc":"2.0","method":"chat.chunk","params":{"text":"..."}}
 *   - {"jsonrpc":"2.0","method":"chat.done","params":{...}}
 *   - {"jsonrpc":"2.0","id":1,"error":{...}}              ← bei Fehler
 */

"use strict";

const http = require("http");
const https = require("https");
const { URL } = require("url");

// --- JSON-RPC I/O ----------------------------------------------------------

const stdinChunks = [];
process.stdin.setEncoding("utf8");
process.stdin.on("data", (chunk) => {
  stdinChunks.push(chunk);
  // Versuche, jede Zeile als einzelne Nachricht zu parsen.
  const buf = stdinChunks.join("");
  const lines = buf.split("\n");
  // Letzte (möglicherweise unvollständige) Zeile zurückbehalten.
  stdinChunks.length = 0;
  stdinChunks.push(lines.pop() ?? "");
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    handleLine(trimmed).catch((err) => {
      logError("handler crashed:", err);
    });
  }
});
process.stdin.on("end", () => {
  // Flush: letzte Zeile ohne Newline verarbeiten.
  const tail = stdinChunks.join("").trim();
  if (tail) {
    handleLine(tail).catch((err) => logError("handler crashed:", err));
  }
});

function logError(...args) {
  process.stderr.write("[copilot-cli] " + args.map(String).join(" ") + "\n");
}

function writeJson(obj) {
  try {
    process.stdout.write(JSON.stringify(obj) + "\n");
  } catch (err) {
    logError("failed to serialize:", err);
  }
}

// --- Request-Handler -------------------------------------------------------

async function handleLine(line) {
  let msg;
  try {
    msg = JSON.parse(line);
  } catch (err) {
    logError("invalid JSON on stdin:", err.message, "line=", line.slice(0, 200));
    writeJson({
      jsonrpc: "2.0",
      id: null,
      error: { code: -32700, message: "Parse error: " + err.message },
    });
    return;
  }

  const { id, method, params } = msg;
  if (!method) {
    writeJson({
      jsonrpc: "2.0",
      id: id ?? null,
      error: { code: -32600, message: "Invalid Request: missing method" },
    });
    return;
  }

  try {
    switch (method) {
      case "chat":
        await handleChat(id, params ?? {});
        break;
      default:
        writeJson({
          jsonrpc: "2.0",
          id,
          error: {
            code: -32601,
            message: `Method not found: ${method}`,
          },
        });
    }
  } catch (err) {
    logError("method", method, "failed:", err);
    writeJson({
      jsonrpc: "2.0",
      id,
      error: {
        code: -32000,
        message: err && err.message ? err.message : String(err),
      },
    });
  }
}

async function handleChat(id, params) {
  const {
    message,
    model,
    api_key,
    endpoint,
    system_prompt,
    temperature,
  } = params;

  if (!message || typeof message !== "string") {
    throw new Error("params.message is required (string)");
  }
  if (!model) throw new Error("params.model is required");
  if (!api_key) throw new Error("params.api_key is required");
  if (!endpoint) throw new Error("params.endpoint is required");

  // Erstes Acknowledge (Response auf den Request) — die Rust-Bridge
  // schmeißt das weg (siehe parse_jsonrpc_stream in bridge.rs).
  writeJson({
    jsonrpc: "2.0",
    id,
    result: { ok: true, started_at: new Date().toISOString() },
  });

  const messages = [];
  if (system_prompt) {
    messages.push({ role: "system", content: system_prompt });
  }
  messages.push({ role: "user", content: message });

  const body = JSON.stringify({
    model,
    messages,
    stream: true,
    ...(typeof temperature === "number" ? { temperature } : {}),
  });

  const url = new URL(endpoint.replace(/\/+$/, "") + "/chat/completions");
  const isHttps = url.protocol === "https:";
  const lib = isHttps ? https : http;

  const headers = {
    "Content-Type": "application/json",
    "Content-Length": Buffer.byteLength(body),
    Authorization: "Bearer " + api_key,
    Accept: "text/event-stream",
  };

  const reqOpts = {
    method: "POST",
    hostname: url.hostname,
    port: url.port || (isHttps ? 443 : 80),
    path: url.pathname + url.search,
    headers,
    timeout: 120_000,
  };

  await new Promise((resolve) => {
    const req = lib.request(reqOpts, (res) => {
      if (res.statusCode < 200 || res.statusCode >= 300) {
        let errBuf = "";
        res.setEncoding("utf8");
        res.on("data", (c) => (errBuf += c));
        res.on("end", () => {
          logError("HTTP", res.statusCode, errBuf.slice(0, 500));
          writeJson({
            jsonrpc: "2.0",
            method: "chat.error",
            params: {
              status: res.statusCode,
              body: errBuf.slice(0, 4000),
            },
          });
          writeJson({
            jsonrpc: "2.0",
            method: "chat.done",
            params: { ok: false, status: res.statusCode },
          });
          resolve();
        });
        return;
      }

      res.setEncoding("utf8");
      let buffer = "";
      let totalChars = 0;
      let finishReason = null;

      res.on("data", (chunk) => {
        buffer += chunk;
        let idx;
        while ((idx = buffer.indexOf("\n")) !== -1) {
          const raw = buffer.slice(0, idx).trim();
          buffer = buffer.slice(idx + 1);
          if (!raw || !raw.startsWith("data:")) continue;
          const payload = raw.slice(5).trim();
          if (payload === "[DONE]") continue;
          let evt;
          try {
            evt = JSON.parse(payload);
          } catch {
            continue;
          }
          const choice = evt.choices && evt.choices[0];
          if (!choice) continue;
          const delta = choice.delta || choice.message || {};
          const text = delta.content;
          if (typeof text === "string" && text.length > 0) {
            totalChars += text.length;
            writeJson({
              jsonrpc: "2.0",
              method: "chat.chunk",
              params: { text },
            });
          }
          if (typeof choice.finish_reason === "string") {
            finishReason = choice.finish_reason;
          }
        }
      });

      res.on("end", () => {
        writeJson({
          jsonrpc: "2.0",
          method: "chat.done",
          params: {
            ok: true,
            finish_reason: finishReason,
            chars: totalChars,
          },
        });
        resolve();
      });

      res.on("error", (err) => {
        logError("response stream error:", err);
        writeJson({
          jsonrpc: "2.0",
          method: "chat.error",
          params: { message: err.message },
        });
        resolve();
      });
    });

    req.on("error", (err) => {
      logError("request error:", err);
      writeJson({
        jsonrpc: "2.0",
        method: "chat.error",
        params: { message: err.message },
      });
      writeJson({
        jsonrpc: "2.0",
        method: "chat.done",
        params: { ok: false },
      });
      resolve();
    });

    req.on("timeout", () => {
      logError("request timeout");
      req.destroy(new Error("Upstream request timeout (120s)"));
    });

    req.write(body);
    req.end();
  });
}