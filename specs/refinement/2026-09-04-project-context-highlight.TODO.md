# Evidenziare `+progetto` e `@contesto` (sintassi todo.txt)

Stato: proposta, da implementare. Seconda delle due spec richieste
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

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — zero modifiche Rust, verificati
comunque per la regola di CLAUDE.md. Verifica manuale in `npm run
tauri dev`: scrivere `+progetto` e `@contesto` in un blocco, colore
sap applicato, testo del blocco invariato dopo un giro di
salvataggio/ricaricamento (nessuna modifica al parser: già garantito
per costruzione, ma verificabile a occhio).
