// Rispecchia le struct Rust esposte dai command Tauri (src-tauri/src/commands.rs
// e crates/ramus-core/src/{block,config}.rs). Tenere allineato a mano.

export interface Block {
  content: string;
  children: Block[];
}

export interface Page {
  path: string;
  /** Dal front-matter (solo pagine, mai journal). `null` se assente. */
  title: string | null;
  blocks: Block[];
}

export interface PageSummary {
  slug: string;
  title: string;
}

export type Theme = "light" | "dark" | "system";

export interface Config {
  vault_path: string;
  theme: Theme;
  search_shortcut: string;
  git_sync_interval_minutes: number;
}

export interface VaultStats {
  journal_count: number;
  page_count: number;
}

export interface Backlink {
  source_path: string;
  source_title: string | null;
  block_content: string;
}

export interface SearchHit {
  path: string;
  kind: "page" | "journal";
  title: string | null;
  snippet_html: string;
}

export type SyncState = "disabled" | "noremote" | "idle" | "syncing" | "conflict" | "offline";

export interface SyncStatus {
  enabled: boolean;
  /** Epoch secondi, `null` se nessun commit esiste ancora. */
  last_commit_at: number | null;
  dirty: boolean;
  state: SyncState;
}
