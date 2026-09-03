# "Vai a data" diventa un'azione digitata nella command palette

Stato: implementata. Quinta spec della fase di refinement. Supera la decisione presa in
`specs/refinement/2026-09-03-statusbar-icons.DONE.md` (che teneva il
date-picker nativo, ridotto a icona, esplicitamente fuori scope
un'azione testuale in command palette) — l'utente ha chiesto di
eliminare del tutto il date-picker e spostare la funzione lì.

## Motivazione

Il date-picker nativo compattato a icona (spec precedente) resta un
elemento a parte nella status bar, con la fragilità nota degli
pseudo-elementi `::-webkit-datetime-edit-*`. L'utente preferisce
eliminarlo e digitare direttamente una data nella command palette,
già il punto d'accesso per "Vai a oggi" e per la ricerca.

## Riconoscimento del formato

Confermato con l'utente: **ISO e italiano**. La palette riconosce sia
`2026-08-15` sia `15/08/2026` (anche con `-` come separatore:
`15-08-2026`). Data non valida (es. 31 aprile) o futura (stesso limite
del vecchio `max={oggi}`) → non riconosciuta, nessuna azione mostrata,
nessun errore: il campo resta una ricerca normale.

## Modifiche

**`src/lib/journal.ts`**:
- Nuova funzione pura `parseTypedDate(input: string): string | null`:
  prova prima il pattern ISO (`/^(\d{4})-(\d{1,2})-(\d{1,2})$/`), poi
  quello italiano (`/^(\d{1,2})[/-](\d{1,2})[/-](\d{4})$/`); valida i
  componenti costruendo un `Date` locale e verificando che non ci sia
  stato un roll-over (`new Date(2026, 3, 31)` per un 31 aprile
  diventerebbe silenziosamente 1 maggio — round-trip check per
  rifiutarlo invece di accettare una data sbagliata); rifiuta le date
  future confrontando le stringhe ISO risultanti (confronto
  lessicografico, stesso principio già in uso nel resto del file).
- `formatPrettyDate` diventa `export` (già esiste, usata da
  `formatJournalHeader`): riusata per l'etichetta dell'azione ("15
  agosto" o "15 agosto 2026" se anno diverso da quello corrente,
  stessa regola già in uso).

**`CommandPalette.tsx`**:
- `PaletteItem` guadagna il kind `{ kind: "date"; iso: string }`.
  `SECTION_LABELS` guadagna `date: "Data"` (sezione propria, non
  mescolata con "Azioni": è un match ad alta confidenza sull'intero
  testo digitato, non un'azione fissa dalla lista).
- In `items` (dentro `useMemo`), quando `trimmed` non è vuoto: prova
  `parseTypedDate(trimmed)`; se valido, l'item `{ kind: "date", iso
  }` va **in cima alla lista**, prima delle azioni e dei risultati di
  ricerca — se l'intero input è una data valida è quasi certamente
  quello che si vuole, più affidabile di un match fuzzy sul testo.
- Rendering dell'item: `Vai al {formatPrettyDate(iso)}` (stesso
  pattern testuale delle altre righe della palette).

**`App.tsx`** (`handlePaletteSelect`): nuovo ramo per `item.kind ===
"date"`, stesso trattamento già riservato ai risultati di ricerca sui
journal (`kind === "hit" && item.hit.kind === "journal"`): se la vista
corrente è una pagina, prima `returnToJournal()`, poi
`jumpToDate(item.iso)`.

**Eliminati** (il date-picker sparisce, non resta nulla da
compattare):
- `src/components/JournalControls.tsx` — dopo aver tolto "Oggi"
  (spec precedente) l'unico contenuto rimasto era l'`<input
  type="date">`; tolto anche quello, il componente non avrebbe più
  nulla da renderizzare.
- In `App.tsx`, il blocco `{view.kind === "journal" && (<JournalControls
  .../>)}` nella status bar.
- In `index.css`: `.journal-controls`, `.journal-date-picker` e le sue
  regole `::-webkit-datetime-edit-*` / `::-webkit-calendar-picker-indicator`
  (introdotte nella spec precedente, mai arrivate in produzione oltre
  questa sessione di sviluppo).

Status bar dopo questa spec: solo ⚙ Impostazioni e, condizionale, il
badge di sync — la navigazione a una data (oggi o arbitraria) passa
interamente dalla command palette.

## Fuori scope

- Riconoscere date relative testuali ("ieri", "domani", "lunedì
  scorso"): la palette già copre "oggi" con l'azione fissa "Vai a
  oggi"; formati relativi aggiuntivi sono un'estensione a sé,
  proposta solo se servisse davvero.
- Suggerimenti/autocomplete mentre si digita una data parziale (es.
  "15/08" senza anno): fuori scope, l'utente digita la data per
  intero.

## Domande aperte

Nessuna: formato di riconoscimento (ISO + italiano) confermato
dall'utente prima di scrivere questa spec.

## Test da scrivere

Nessun test automatico nuovo: coerente con l'assenza di un runner JS
per componenti/funzioni pure nel progetto (stessa scelta già presa per
tutto il codice frontend in `specs/`). `parseTypedDate` è comunque
scritta come funzione pura isolata (stesso principio già seguito per
`mcp_disabled_message` lato Rust) proprio per essere testabile in
futuro se un runner venisse introdotto.

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — tutti puliti (zero modifiche Rust).
`grep` di conferma che nessun file importa più `JournalControls` dopo
la cancellazione. `parseTypedDate` verificata per davvero con un
piccolo script Node isolato (stessa logica, fuori dal progetto): ISO e
italiano riconosciuti, 1-2 cifre per giorno/mese accettate in entrambi
i formati, 31 aprile e 31/04 rifiutati (niente roll-over silenzioso),
data futura rifiutata, testo non-data rifiutato. Non verificato in
questa sessione: il giro reale in `npm run tauri dev` (digitare una
data nella palette e vedere il salto avvenire) — lasciato all'utente.
