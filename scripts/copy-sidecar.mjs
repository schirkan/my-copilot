// Kopiert die Copilot-CLI-Sidecar-Binary aus
// src-tauri/binaries/copilot-<TRIPLE>.exe in alle
// target/<profile>/binaries/ Verzeichnisse, die Tauri 2 in
// `tauri-plugin-shell` zur Laufzeit auflöst (resource_dir).
//
// Ausgeführt vor `tauri dev` / `tauri build` via npm-Skript.

import { existsSync, mkdirSync, copyFileSync, statSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, '..');
const srcTauriDir = join(repoRoot, 'src-tauri');
const srcDir = join(srcTauriDir, 'binaries');

// Host-Triple (cargo liefert das ueber `rustc -vV`).
import { execSync } from 'node:child_process';
const triple = process.env.TARGET_TRIPLE
  || (() => {
    try {
      const out = execSync('rustc -vV', { encoding: 'utf8' });
      const m = out.match(/^host:\s*(\S+)/m);
      if (m) return m[1];
    } catch { }
    return 'x86_64-pc-windows-msvc';
  })();

const exeName = `copilot-${triple}.exe`;
const srcPath = join(srcDir, exeName);

if (!existsSync(srcPath)) {
  console.error(`[sidecar] FEHLT: ${srcPath}`);
  console.error('[sidecar] BYOK-Chat wird fehlschlagen. Lege die echte GitHub Copilot CLI-Binary dort ab.');
  process.exit(1);
}

const targetDir = join(srcTauriDir, 'target');
if (!existsSync(targetDir)) {
  console.log(`[sidecar] target/ fehlt noch — wird beim ersten cargo-Build angelegt. OK.`);
  process.exit(0);
}

// Alle Profile (debug, release, custom) unter target/ auflisten.
for (const profile of readdirSync(targetDir)) {
  const profileDir = join(targetDir, profile);
  try {
    if (!statSync(profileDir).isDirectory()) continue;
  } catch {
    continue;
  }
  const dstDir = join(profileDir, 'binaries');
  const dstPath = join(dstDir, exeName);

  let needsCopy = true;
  try {
    const s = statSync(srcPath);
    const d = statSync(dstPath);
    needsCopy = s.mtimeMs > d.mtimeMs;
  } catch {
    needsCopy = true;
  }

  if (needsCopy) {
    mkdirSync(dstDir, { recursive: true });
    copyFileSync(srcPath, dstPath);
    console.log(`[sidecar] ${profile}/binaries/${exeName} aktualisiert`);
  } else {
    console.log(`[sidecar] ${profile}/binaries/${exeName} bereits aktuell`);
  }
}
