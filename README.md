# Ramus

App desktop di journaling — outliner a blocchi su file markdown locali,
compatibili con Obsidian. Tauri v2 + Rust (core puro, testabile da solo)
+ React/TypeScript.

**Fase 1 completa** (milestone M1-M6, vedi [`SPEC.md`](./SPEC.md) per il
dettaglio): journal giornaliero con outliner a blocchi, link/tag e ricerca
full-text, sync Git automatica, rifiniture UI (command palette, scorciatoie
configurabili, task nei blocchi), un server MCP per collegare un agente AI
al vault, e le fondamenta architetturali per il supporto mobile. Ogni spec
puntuale è in [`specs/`](./specs/), organizzata per milestone.

Il progetto entra ora nella fase di refinement: test dall'uso reale,
correzioni, rifiniture — non più costruzione per milestone.

## Server MCP

`ramus-mcp` (`crates/ramus-mcp`) espone il vault in lettura/scrittura a un
client MCP (Claude Code, Claude Desktop) via stdio, come processo
indipendente dalla GUI:

```bash
cargo build -p ramus-mcp
./target/debug/ramus-mcp --print-config   # snippet pronto da incollare nel client
```

Si abilita o disabilita dalla sezione MCP di Impostazioni, dentro l'app.

## Documentazione

- [`SPEC.md`](./SPEC.md) — specifica di progetto, milestone, formato su disco
- [`CLAUDE.md`](./CLAUDE.md) — convenzioni operative e comandi (`npm run tauri dev`, `cargo test`, ecc.)
- [`specs/`](./specs/) — spec di dettaglio per ogni feature

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
