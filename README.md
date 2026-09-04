<p align="center">
  <img src="assets/logo.svg" width="72" height="72" alt="Ramus logo" />
</p>

<h1 align="center">Ramus</h1>

<p align="center">
  A local-first, block-outliner journal — for people and for their AI agents.
</p>

<p align="center">
  <a href="./LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-2F6B4F.svg"></a>
  <a href="https://github.com/calca/ramus/actions/workflows/test.yml"><img alt="Tests" src="https://github.com/calca/ramus/actions/workflows/test.yml/badge.svg"></a>
  <img alt="Platforms" src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux%20%7C%20Android-8A857C.svg">
</p>

---

Ramus is a desktop journaling app: a daily journal, written as nested
outliner blocks, saved as plain Markdown files on your own disk —
compatible with [Obsidian](https://obsidian.md) and any other
Markdown-based tool. No account, no cloud, no lock-in. Your journal is
a folder of `.md` files you can read in ten years with nothing but a
text editor.

It's also built for a world where you might want an AI agent working
*with* your notes, not just reading a chat log about them: Ramus ships
an [MCP](https://modelcontextprotocol.io) server, so tools like Claude
Code or Claude Desktop can search, read, and write directly to your
vault — on your terms, only when you've connected them yourself.

The name comes from the Latin for "branch" — a nod to the outliner's
tree of blocks, and to Stecco, the little branch-creature who lives in
the About panel.

## Why Ramus

- **Local-first, genuinely.** Every note is a `- ` bulleted line in a
  Markdown file, one block per line, two spaces per indent level — a
  format documented and fixed on purpose, so it stays readable and
  diffable outside the app.
- **A daily journal, not a folder of documents.** Open the app and
  land straight on today, an infinite scroll of past days below it —
  the same model as Logseq, built from scratch on top of a hand-rolled
  block parser (deliberately no third-party Markdown library, so the
  round-trip between disk and editor stays exact).
- **`[[Links]]`, `#tags`, full-text search, backlinks.** A small SQLite
  index and a `tantivy` search index, both fully regenerable from the
  Markdown files — the files are always the source of truth.
- **Sync without a platform.** Optional Git-based sync, local-only or
  pushed to any remote you already have — no Ramus account, ever.
  Even local-only, it gives you a real commit history to recover from
  a bad write.
- **AI-native, via MCP.** A separate `ramus-mcp` binary exposes
  read/write tools over your vault to any MCP client — independent
  from the GUI, opt-in, with a kill switch in Settings.
- **Fast and small.** Tauri + Rust, not Electron — a native binary,
  not a bundled Chromium.
- **Keyboard-first.** A command palette, configurable shortcuts, and a
  cheatsheet that's always one keystroke away.
- **Tuned for reading.** A palette calibrated for long sessions (light
  and dark), a serif body font on Apple platforms, and light/dark/
  system theming that follows your OS.
- **Italian and English**, with more straightforward to add — see
  [`src/i18n/`](./src/i18n/).
- **Android**, for real: `tauri android init` and a debug build both
  succeed, with a CI workflow that produces a real APK on every
  release tag. iOS foundations are laid but not yet built.

## Getting started

There's no signed, downloadable release yet (see
[`specs/release/`](./specs/release/) for what's still open) — for now,
build it yourself:

```bash
git clone https://github.com/calca/ramus.git
cd ramus
npm install
npm run tauri dev     # launches the app, creates ~/Journal on first run
```

Nothing to configure: the first run creates a vault and opens today's
journal, no prompts. `npm run tauri build` produces a release binary
for your platform; see [`specs/release/`](./specs/release/) for the
Android APK pipeline.

## Connecting an AI agent

```bash
cargo build -p ramus-mcp
./target/debug/ramus-mcp --print-config   # ready-to-paste client config
```

Paste the snippet into `.mcp.json` (Claude Code) or
`claude_desktop_config.json` (Claude Desktop), or copy it straight
from the MCP section of Ramus's own Settings. Add `--read-only` to
expose only search and reading, never writes.

## Documentation

- [`SPEC.md`](./SPEC.md) — project specification, milestones, on-disk
  format
- [`CLAUDE.md`](./CLAUDE.md) — operating conventions for anyone (human
  or AI) working on the codebase
- [`specs/`](./specs/) — one written spec per feature, confirmed
  before implementation, kept as a running design log — organized by
  milestone (`M1`–`M7`), plus `refinement/` and `release/` for work
  outside the milestone plan

Most of the code, commit history, and detailed specs are in Italian —
the project's original working language. Contributions and issues in
English are just as welcome.

## Tech stack

Tauri v2 + Rust on the backend (`ramus-core` is a dependency-minimal,
Tauri-free crate — pure parser, block model, vault, and index, fully
testable on its own), React + TypeScript on the frontend, Tiptap for
the block editor, SQLite for the link/tag index, `tantivy` for
full-text search, `git2` for sync, and the official `rmcp` SDK for the
MCP server.

## License

[MIT](./LICENSE)
