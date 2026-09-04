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
  /** Chiave = id azione stabile ("command_palette", "cheatsheet", ...). */
  shortcuts: Record<string, string>;
  git_sync_interval_minutes: number;
  task_rollover_enabled: boolean;
  task_rollover_days: number;
  mcp_enabled: boolean;
}

export interface RolloverOutcome {
  moved_count: number;
}

export interface McpInfo {
  enabled: boolean;
  binary_found: boolean;
  /** Snippet JSON pronto da incollare, `null` se `binary_found` è `false`. */
  config_snippet: string | null;
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

export interface TaskHit {
  path: string;
  kind: "page" | "journal";
  title: string | null;
  content: string;
}

export type SyncState = "disabled" | "noremote" | "idle" | "syncing" | "conflict" | "offline";

export interface SyncStatus {
  enabled: boolean;
  /** Epoch secondi, `null` se nessun commit esiste ancora. */
  last_commit_at: number | null;
  dirty: boolean;
  state: SyncState;
}
