// Rispecchia le struct Rust esposte dai command Tauri (src-tauri/src/commands.rs
// e crates/ramus-core/src/{block,config}.rs). Tenere allineato a mano.

export interface Block {
  content: string;
  children: Block[];
}

export interface Page {
  path: string;
  blocks: Block[];
}

export interface Config {
  vault_path: string;
}
