// Wrapper tipizzati sui command Tauri. Il frontend non tocca mai il
// filesystem direttamente: passa solo path relativi al vault.

import { invoke } from "@tauri-apps/api/core";

import type {
  Backlink,
  Block,
  Config,
  Locale,
  McpInfo,
  Page,
  PageSummary,
  RolloverOutcome,
  SearchHit,
  SyncStatus,
  TaskHit,
  Theme,
  VaultStats,
} from "./types";

export function getConfig(): Promise<Config> {
  return invoke<Config>("get_config");
}

export function setVaultPath(path: string): Promise<Config> {
  return invoke<Config>("set_vault_path", { path });
}

export function openToday(): Promise<Page> {
  return invoke<Page>("open_today");
}

/** Sposta i task `[ ] ` non fatti rimasti nella finestra configurata
 * verso oggi — no-op se `task_rollover_enabled` è `false`. Da chiamare
 * prima di `openToday()`, non dopo (vedi App.tsx). */
export function rollOverUnfinishedTasks(): Promise<RolloverOutcome> {
  return invoke<RolloverOutcome>("roll_over_unfinished_tasks");
}

export function readPage(path: string): Promise<Page> {
  return invoke<Page>("read_page", { path });
}

export function writePage(path: string, blocks: Block[]): Promise<void> {
  return invoke<void>("write_page", { path, blocks });
}

/** Journal esistenti, più recente prima, strettamente precedenti a
 * `before` (data ISO 8601, o `null` per partire dal più recente). */
export function listJournals(before: string | null, limit: number): Promise<Page[]> {
  return invoke<Page[]>("list_journals", { before, limit });
}

/** Apre la dialog nativa "scegli cartella". `null` se l'utente annulla. */
export function pickVaultFolder(): Promise<string | null> {
  return invoke<string | null>("pick_vault_folder");
}

export function vaultStats(): Promise<VaultStats> {
  return invoke<VaultStats>("vault_stats");
}

export function setTheme(theme: Theme): Promise<Config> {
  return invoke<Config>("set_theme", { theme });
}

export function setLocale(locale: Locale): Promise<Config> {
  return invoke<Config>("set_locale", { locale });
}

export function listPages(): Promise<PageSummary[]> {
  return invoke<PageSummary[]>("list_pages");
}

/** Apre (creando se non esiste) la pagina identificata da `name`. */
export function openPage(name: string): Promise<Page> {
  return invoke<Page>("open_page", { name });
}

/** Blocchi che linkano a `targetTitle`, in qualunque pagina o journal. */
export function findBacklinks(targetTitle: string): Promise<Backlink[]> {
  return invoke<Backlink[]>("find_backlinks", { targetTitle });
}

/** Tag distinti presenti nel vault, in ordine alfabetico. */
export function listTags(): Promise<string[]> {
  return invoke<string[]>("list_tags");
}

/** Tutti i task "[ ] " aperti nel vault, in qualunque journal o pagina,
 * ordinati per path. */
export function listOpenTasks(): Promise<TaskHit[]> {
  return invoke<TaskHit[]>("list_open_tasks");
}

/** Ricerca full-text su titolo + contenuto di pagine e journal. */
export function search(query: string): Promise<SearchHit[]> {
  return invoke<SearchHit[]>("search", { query });
}

export function setShortcut(actionId: string, shortcut: string): Promise<Config> {
  return invoke<Config>("set_shortcut", { actionId, shortcut });
}

/** Crea il repository Git (idempotente) e committa subito lo stato
 * attuale del vault. */
export function initGitSync(): Promise<SyncStatus> {
  return invoke<SyncStatus>("init_git_sync");
}

export function getSyncStatus(): Promise<SyncStatus> {
  return invoke<SyncStatus>("get_sync_status");
}

export function setGitSyncInterval(minutes: number): Promise<Config> {
  return invoke<Config>("set_git_sync_interval", { minutes });
}

export function setTaskRollover(enabled: boolean, days: number): Promise<Config> {
  return invoke<Config>("set_task_rollover", { enabled, days });
}

export function setMcpEnabled(enabled: boolean): Promise<Config> {
  return invoke<Config>("set_mcp_enabled", { enabled });
}

export function getMcpInfo(): Promise<McpInfo> {
  return invoke<McpInfo>("get_mcp_info");
}

/** Imposta (o aggiorna) il remote "origin" e prova subito un pull. */
export function setGitRemote(url: string): Promise<SyncStatus> {
  return invoke<SyncStatus>("set_git_remote", { url });
}

