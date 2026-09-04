# "Task aperti": lista di tutti i task non fatti nel vault

Stato: proposta, da implementare. Prima delle due spec richieste
("cosa manca rispetto a todo.txt" → 1) lista di tutti i task aperti,
2) evidenziare `+progetto`/`@contesto`).

## Motivazione

Oggi un task (`[ ] testo`) esiste solo dov'è stato scritto — nessun
modo di vederli tutti insieme. Il rollover automatico (M4,
`rollover.rs`) aiuta ma copre solo una parte: scansiona esclusivamente
`journals/` (mai `pages/`) e solo gli ultimi N giorni configurati
(`task_rollover_days`) — un task dimenticato più indietro, o scritto
in una pagina invece che nel journal, resta invisibile per sempre a
meno di scorrere a mano o azzeccare la ricerca full-text giusta.

**Non è una query language** (SPEC.md, "Fuori scope"): un'unica
operazione a scopo fisso — "tutti i blocchi che iniziano con `[ ] `" —
stessa categoria di `find_backlinks`/`list_tags`, già esistenti.
Nessun filtro, nessuna sintassi di ricerca, nessuna configurazione:
si apre, si vede la lista, si clicca per andare al task.

## Dove vive il dato: già nell'indice, nessuna scansione nuova

La tabella `blocks` di `crates/ramus-core/src/index.rs` memorizza già
il `content` testuale di ogni blocco insieme al `page_id` (join su
`pages` per `path`/`title`/`kind`) — sincronizzata a ogni `Index::sync`
esistente, non serve rileggere file da disco. Un task aperto è
semplicemente un blocco il cui `content` inizia con `"[ ] "` — `LIKE
'[ ] %'` in SQLite fa match letterale su `[`/`]` (non sono wildcard
LIKE, solo `%`/`_` lo sono), nessuna regex necessaria.

## Modifiche

**`crates/ramus-core/src/index.rs`**:
```rust
pub struct TaskHit {
    pub path: String,
    pub kind: String, // "journal" | "page", da pages.kind
    pub title: Option<String>,
    pub content: String,
}

/// Blocchi "[ ] ..." in tutto il vault, in qualunque pagina — journal
/// o pagina, qualunque età (rollover copre solo journals/ e una
/// finestra di N giorni, questo copre tutto). Ordinati per path
/// (i journal, nominati per data ISO, escono in ordine cronologico).
pub fn list_open_tasks(&self) -> Result<Vec<TaskHit>, CoreError> {
    // SELECT blocks.content, pages.path, pages.kind, pages.title
    // FROM blocks JOIN pages ON pages.id = blocks.page_id
    // WHERE blocks.content LIKE '[ ] %'
    // ORDER BY pages.path
}
```
Stesso pattern esatto di `find_backlinks`/`list_tags` (query preparata,
`query_map`, niente di nuovo architetturalmente).

**`src-tauri/src/commands.rs`**: `#[tauri::command] pub fn
list_open_tasks(state) -> Result<Vec<TaskHit>, CoreError>` — wrapper
sottile, `lock_index(&state)?.list_open_tasks()`.

**`src/lib/types.ts`**: `TaskHit` (stessa forma di `SearchHit` meno
`snippet_html`: `path`, `kind: "page" | "journal"`, `title`,
`content`) — riuso diretto della stessa logica di navigazione già
scritta per i risultati di ricerca (`formatJournalHeader` +
`journalDateFromPath` se `kind === "journal"`, altrimenti titolo/path).

**Nuovo componente `OpenTasksPanel.tsx`** (modellato su
`Cheatsheet.tsx`: stesso `Modal`, stesso bottone di chiusura) — **non**
riusa `.settings-panel-header` + `.settings-section` diretti dentro
`.settings-panel`: sospetto (non confermato) che quella combinazione
abbia la stessa regressione di padding appena risolta per
`CommandPalette` (vedi
`specs/refinement/2026-09-04-palette-padding.DONE.md`, "Scoperta
collaterale"). Invece: `panelClassName="tasks-panel"` dedicato (stessa
tecnica additiva già usata per `.palette-panel`), con padding proprio
e altezza che si adatta al contenuto fino a un tetto massimo. Click su
un task naviga alla pagina sorgente (stesso `navigateToPage`/
`jumpToDate` già usato per i risultati di ricerca) — **non** un modo
di segnare il task fatto da qui: resterebbe da aprire la pagina
comunque per vedere il contesto, coerente con "sola lettura, poi vai
al blocco" già scelto per backlink/ricerca.

**Command palette**: nuova azione fissa in `paletteActions.ts`
("Task aperti", id `"open-tasks"`) accanto alle altre cinque — stesso
trattamento, non configurabile.

**`ramus-mcp`**: nuovo tool di sola lettura `list_open_tasks`,
speculare a `list_tags`/`find_backlinks` già esistenti (stesso
principio: ogni operazione di lettura del core diventa un tool MCP,
M5) — un agente collegato può vedere i task aperti tanto quanto la
GUI.

## Fuori scope

- Qualunque filtro (per data, per pagina, per `+progetto`/`@contesto`
  una volta esistenti — spec 2): lista completa, senza eccezioni, non
  vale ancora "query language" ma un filtro parametrico ci si
  avvicinerebbe.
- Segnare un task fatto direttamente dalla lista: solo navigazione,
  vedi sopra.
- Un limite/paginazione sulla lista: nessun cap per ora, come
  `list_tags`. Se un vault molto vecchio la rendesse scomoda, spec a
  parte.

## Domande aperte

Nessuna: le scelte sopra (nessun filtro, sola navigazione, nessun
limite, parità MCP) seguono direttamente precedenti già stabiliti nel
progetto, non bivi di prodotto.

## Test da scrivere

**Core** (`index.rs`): `list_open_tasks_finds_tasks_across_journals_and_pages`,
`list_open_tasks_excludes_done_tasks` (`[x] `/`[X] ` non compaiono),
`list_open_tasks_on_empty_vault_is_empty`, `list_open_tasks_orders_by_path`.
**`ramus-mcp`**: un test analogo ai read tool esistenti (vault con un
task aperto e uno fatto, verifica che solo il primo compaia).
**Frontend**: nessuno, coerente con l'assenza di test per componenti
React nel progetto.

## Verifica

`cargo test`, `cargo clippy --all-targets -D warnings`, `cargo fmt
--check`, `npm run typecheck`, `npm run test` — tutti puliti prima di
chiudere. Verifica manuale in `npm run tauri dev`: azione "Task
aperti" dalla palette, lista corretta con task da journal e da
pagina, click naviga al punto giusto.
