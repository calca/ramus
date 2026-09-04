# Evidenziare `+progetto` e `@contesto` (sintassi todo.txt)

Stato: implementata. Seconda delle due spec richieste
("cosa manca rispetto a todo.txt").

## Motivazione

`[[link]]` e `#tag` sono già riconosciuti e colorati mentre si scrive
(`linkTagHighlight.ts`), ma solo decorazione — il testo del blocco
resta esattamente quello digitato, nessun mark/nodo, nessuna modifica
al parser/serializer. `+progetto`/`@contesto` (convenzione todo.txt)
oggi non hanno alcun trattamento visivo: stesso principio, stessa
implementazione, solo due pattern in più.

## Modifiche

**`src/editor/linkTagHighlight.ts`**: due nuovi pattern accanto a
`LINK_PATTERN`/`TAG_PATTERN`, stesso identico trattamento (`matchAll`
sul testo del nodo, `Decoration.inline` con una classe):
```ts
const PROJECT_PATTERN = /\+[\w-]+/g;
const CONTEXT_PATTERN = /@[\w-]+/g;
```
Nuove classi `editor-project`/`editor-context` (non riusare
`editor-tag`: stesso colore ma pattern diverso, tenerle distinte in
CSS costa nulla e permette di ritoccarle indipendentemente in
futuro).

**`index.css`**: `.editor-project`, `.editor-context` — stesso
`color: var(--ramus-sap)` di link/tag (mai amber: riservato a giorno
corrente/focus, regola già in vigore), nessun `cursor: pointer` (né
tag né project/context sono cliccabili oggi — nessun comportamento
nuovo, solo colore).

**Non generale come `+`/`@` liberi**: stesso limite già accettato per
`#tag` — un `@` o un `+` in mezzo a prosa normale (un'email, della
matematica) viene comunque colorato. Non un problema nuovo introdotto
qui, lo stesso trade-off già in produzione per i tag.

## Fuori scope

- Riconoscerli ovunque tranne che nei task, o viceversa solo nei task:
  stesso trattamento di `#tag`, validi in qualunque blocco — non
  serve sapere se il blocco è un task per decorarli.
- Click per filtrare/cercare per progetto o contesto: implicherebbe
  una qualche forma di query, fuori scope per lo stesso motivo già
  scritto in `specs/refinement/2026-09-04-task-aperti.TODO.md`. Solo
  colore, come i tag oggi.
- Autocomplete per `+`/`@` (esiste già per `[[` e `#`, vedi
  `linkAutocomplete.ts`/`tagAutocomplete.ts`): non richiesto, nessuna
  lista da cui suggerire (i progetti/contesti non sono un'entità
  registrata da nessuna parte, a differenza dei titoli di pagina).

## Domande aperte

Nessuna: stesso pattern già stabilito per `#tag`, applicato a due
sintassi in più.

## Test da scrivere

Nessuno: `linkTagHighlight.ts` è una decorazione ProseMirror
(side-effect sulla view), stesso limite già documentato in
`specs/release/2026-09-03-copertura-test-frontend.DONE.md` per
`linkAutocomplete`/`tagAutocomplete`/`moveBlock` — non nella lista dei
file testabili senza un mock pesante di `EditorView`.

## Verifica

Implementate le due modifiche descritte sopra: `PROJECT_PATTERN`/
`CONTEXT_PATTERN` e i due loop `matchAll` in
`src/editor/linkTagHighlight.ts` (stesso pattern del loop
`TAG_PATTERN` esistente), `.editor-project`/`.editor-context` aggiunte
alla stessa regola `color: var(--ramus-sap)` di `.editor-link`/
`.editor-tag` in `src/index.css` (nessun `cursor: pointer`, nessun
uso di `--ramus-amber`). Zero modifiche Rust.

Checklist eseguita (in un working directory condiviso con un altro
agente che stava implementando in parallelo una feature Rust/TS più
grande — riportati i risultati grezzi, incluso ciò che non riguarda
questa modifica):

- `npm run typecheck`: **pass** (0 errori).
- `npm run test` (vitest): **pass**, 77/77 test, 6 file.
- `cargo clippy --all-targets -- -D warnings`: **pass**, nessun
  warning.
- `cargo test`: **fail**, ma per un motivo indipendente da questa
  modifica — `index::tests::list_open_tasks_finds_tasks_across_journals_and_pages`
  in `crates/ramus-core/src/index.rs` (feature "open tasks" ancora in
  corso da parte dell'altro agente concorrente; nessun file toccato
  da questa spec è coinvolto). Tutti gli altri test passano
  (120 passed, 1 failed).
- `cargo fmt --all -- --check`: **fail**, ma anche qui solo su
  `crates/ramus-core/src/index.rs` e `crates/ramus-mcp/src/main.rs`
  — stessa feature concorrente, non file di questa spec.

Verifica manuale in `npm run tauri dev` non eseguita in questa sessione
(ambiente CI/non interattivo); il comportamento è meccanicamente
identico a quello già in produzione per `#tag` (stesso `matchAll` +
`Decoration.inline`), quindi non richiede una verifica visiva separata
per fiducia nell'implementazione — da fare comunque al primo avvio utile
dell'app.
