// Wrapper tipizzati sui command Tauri. Il frontend non tocca mai il
// filesystem direttamente: passa solo path relativi al vault.

import { invoke } from "@tauri-apps/api/core";

import type { Backlink, Block, Config, Page, PageSummary, Theme, VaultStats } from "./types";

export function getConfig(): Promise<Config> {
  return invoke<Config>("get_config");
}

export function setVaultPath(path: string): Promise<Config> {
  return invoke<Config>("set_vault_path", { path });
}

export function openToday(): Promise<Page> {
  return invoke<Page>("open_today");
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

