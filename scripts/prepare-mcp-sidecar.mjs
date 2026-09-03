#!/usr/bin/env node
// Compila ramus-mcp in release e lo copia in src-tauri/binaries/ con il
// nome suffissato dal target triple che Tauri si aspetta per un
// externalBin (bundle.externalBin in tauri.conf.json) — vedi
// specs/release/2026-09-03-packaging-mcp.DONE.md. Eseguito prima di
// `tauri build` (beforeBuildCommand in tauri.conf.json), mai in dev:
// `tauri dev` continua a trovare ramus-mcp come file fratello in
// target/debug/, invariato.
//
// Script Node invece di uno shell script: deve girare identico su
// macOS/Windows/Linux, niente dipendenza nuova (fs/child_process sono
// già disponibili in un progetto npm).

import { execFileSync } from "node:child_process";
import { copyFileSync, chmodSync, mkdirSync, existsSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const isWindows = process.platform === "win32";

function targetTriple() {
  const output = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  const line = output.split("\n").find((l) => l.startsWith("host: "));
  if (!line) {
    throw new Error("impossibile determinare il target triple da `rustc -vV`");
  }
  return line.replace("host: ", "").trim();
}

const triple = targetTriple();
console.log(`[prepare-mcp-sidecar] target triple: ${triple}`);

execFileSync("cargo", ["build", "-p", "ramus-mcp", "--release"], {
  cwd: repoRoot,
  stdio: "inherit",
});

const exeName = isWindows ? "ramus-mcp.exe" : "ramus-mcp";
const source = join(repoRoot, "target", "release", exeName);
if (!existsSync(source)) {
  throw new Error(`binario compilato non trovato: ${source}`);
}

const binariesDir = join(repoRoot, "src-tauri", "binaries");
mkdirSync(binariesDir, { recursive: true });

const destName = isWindows ? `ramus-mcp-${triple}.exe` : `ramus-mcp-${triple}`;
const dest = join(binariesDir, destName);
copyFileSync(source, dest);
if (!isWindows) {
  chmodSync(dest, 0o755);
}

console.log(`[prepare-mcp-sidecar] scritto ${dest}`);
