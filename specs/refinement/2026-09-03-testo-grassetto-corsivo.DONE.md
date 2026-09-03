# Grassetto e corsivo nel corpo dei blocchi

Stato: implementata. Undicesima spec della fase di refinement.
Ambito confermato dall'utente: solo grassetto e corsivo — barrato e
codice inline restano fuori (eventualmente una spec a parte).

## Motivazione

I marks inline (bold/italic/strike/code) erano disattivati fin da M1:
"il contenuto di un blocco è markdown grezzo come testo semplice"
(`extensions.ts`). Nessun divieto esplicito in SPEC.md ("Fuori
scope"), solo una semplificazione mai rivista. Con Fase 1 chiusa,
grassetto/corsivo è la prima vera funzionalità (non rifinitura visiva)
della fase di refinement — un parser/serializer scritto a mano, non
solo CSS.

## Sintassi: CommonMark, non un formato Ramus-specifico

`**grassetto**`, `*corsivo*`, `***entrambi***` — stessa sintassi di
Obsidian e di qualunque editor CommonMark, per restare coerenti con
l'obiettivo di compatibilità di SPEC.md ("file... apribili e
modificabili da altri editor markdown"). Sottolineato (`_corsivo_`,
`__grassetto__`) **non riconosciuto**, né in scrittura né in lettura —
vedi "Fuori scope".

## `ramus-core` non cambia

`Block.content` è già una stringa opaca ("testo markdown inline",
SPEC.md riga 106): il core Rust non interpreta la sintassi inline, la
tratta come qualunque altro carattere. Grassetto/corsivo sono
interamente un problema del frontend (Tiptap ↔ testo del blocco) — zero
modifiche a `crates/ramus-core`, zero rischio per il round-trip del
parser Rust (`parser.rs`), che infatti non ha bisogno di un test nuovo.

## Nuovo modulo: `src/editor/inlineMarks.ts`

Scritto a mano (CLAUDE.md, "niente librerie markdown per Tiptap, la
serializzazione è scritta a mano di proposito"):

- `escapeInlineText(text): string` — sfugge `\` e `*` letterali
  (`\\`, `\*`) prima di scriverli su disco. Senza, un testo semplice
  come "2*3=6" verrebbe riletto come corsivo aperto al prossimo
  caricamento: è l'unico modo per garantire il round-trip (CLAUDE.md,
  regola 5) anche per testo che contiene questi due caratteri senza
  intenzione di formattazione.
- `parseInlineMarks(source): InlineRun[]` — tokenizza una stringa
  markdown in run di testo con marks (`bold`/`italic`), riconoscendo
  `\\`/`\*` come sequenze di escape e i delimitatori `*`/`**`/`***`.
  **Nessun emphasis nidificato**: un delimitatore aperto si chiude
  alla prima occorrenza di un delimitatore della stessa lunghezza,
  senza scansionare ricorsivamente il contenuto fra i due — vedi
  "Fuori scope".

## Modifiche a serializer/deserializer esistenti

- `serializer.ts` (doc → `Block[]`, usato al salvataggio): ogni text
  node del paragrafo viene sfuggito (`escapeInlineText`) e avvolto
  in base ai suoi `marks` (`**`/`*`/`***`), poi concatenato. Link
  `[[..]]` e tag `#..` restano testo semplice, invariati — non sono
  marks, sono solo caratteri con una decorazione visiva
  (`LinkTagHighlight`), niente di nuovo da coordinare.
- `deserializer.ts` (`Block[]` → doc, usato al caricamento): il
  contenuto grezzo del blocco passa da un singolo text node piatto a
  `parseInlineMarks(content).map(...)` — più text node, ciascuno con i
  propri `marks` quando presenti.
- `pmNode.ts`: `PMNode` guadagna `marks?: PMMark[]` (nuovo tipo
  `PMMark = { type: string }`), il sottoinsieme minimo dello schema
  Tiptap che questi due file consumano.

## `extensions.ts`: bold/italic riabilitati

`StarterKit.configure({ ..., bold: false, italic: false, ... })` →
tolte le due righe (tornano al default StarterKit, quindi abilitati).
`strike`/`code` restano `false`. Scorciatoie: quelle di default di
StarterKit (`Mod-B`/`Mod-I`), **non aggiunte a `SHORTCUT_ACTIONS`**
(registro delle scorciatoie configurabili) — stessa regola già scritta
in `lib/shortcut.ts` per Tab/Invio/Backspace: "solo scorciatoie a
livello finestra entrano in questo registro", grassetto/corsivo sono
scorciatoie dell'editor (keymap ProseMirror), non della finestra.

Nessun bottone/toolbar: solo scorciatoia da tastiera, coerente con il
resto dell'outliner (Tab/Shift-Tab/Mod-Enter, niente chrome visibile
nell'editor).

## Test: aggiunto vitest

Nel progetto non esisteva un runner di test JS — tutte le altre
modifiche frontend di questa fase di refinement erano CSS o logica
di presentazione, verificate a occhio. Questa è la prima volta che
cambia un vero parser/serializer lato frontend, e CLAUDE.md (regola 5)
lo richiede esplicitamente. **Decisione confermata con l'utente**:
aggiungere `vitest` (nuovo devDependency, si integra da solo con Vite
già in uso, zero config necessaria per test di funzioni pure) invece
di continuare con verifica manuale.

- `package.json`: `"vitest": "^5"` in `devDependencies`, script
  `"test": "vitest run"`.
- `CLAUDE.md`: comando aggiunto, checklist "prima di dichiarare
  finito" aggiornata (`npm run test` quando il task tocca
  `src/editor/`).
- `src/editor/inlineMarks.test.ts`: unit test di
  `parseInlineMarks`/`escapeInlineText` in isolamento (bold, italic,
  combinato, run multipli, escape, delimitatore mai chiuso, stringa
  vuota) + round-trip generico escape→parse per una batteria di
  stringhe con `\`/`*` letterali.
- `src/editor/roundtrip.test.ts`: round-trip vero `Block[] → doc →
  Block[]` (testo semplice, grassetto, corsivo, combinato, misto
  semplice/marcato, figli annidati, blocco vuoto, link/tag invariati).

**Nota di design emersa scrivendo i test**: `Block.content` è sempre
sorgente markdown già sfuggita dove serve (mai testo "grezzo così come
appare all'utente") — un test iniziale con un asterisco letterale non
sfuggito falliva, correttamente: non è un bug, è un input non valido
per il modello (nessun percorso reale dell'app produce un `Block` con
un asterisco non sfuggito e zero marks). Corretto il test, non il
codice.

## Fuori scope

- Barrato e codice inline: confermato dall'utente, restano disattivati
  (`strike: false, code: false` invariati).
- Sottolineato come sintassi alternativa per il corsivo (`_testo_`):
  non riconosciuto in lettura. Limite noto — un file scritto altrove
  con quella sintassi resta testo semplice non formattato in Ramus,
  degradazione sicura (nessun crash, nessuna corruzione), non un
  tentativo di copertura completa di CommonMark.
- Emphasis nidificato (`**grassetto *corsivo* grassetto**`): il
  corsivo interno non viene riconosciuto come mark separato — l'intero
  blocco fra i delimitatori più esterni diventa un'unica cosa. Limite
  noto del tokenizer senza ricorsione, accettato per tenere il parser
  semplice e testabile.
- Toolbar/bubble menu per applicare i marks col mouse: solo scorciatoie
  da tastiera, coerente con il resto dell'editor.
- Aggiornare `cargo build`/pipeline Rust per questa modifica: zero
  tocco a `ramus-core`/`src-tauri`, non necessario.

## Verifica

`npm run test` (29 test, tutti nuovi, tutti verdi), `npm run
typecheck`, `cargo test`, `cargo clippy --all-targets -D warnings`,
`cargo fmt --check` — tutti puliti (zero modifiche Rust). Non
verificato in questa sessione con un giro reale in `npm run tauri
dev`: selezionare del testo e premere Cmd+B/Cmd+I, verificare che il
grassetto/corsivo compaia nell'editor e sopravviva a un riavvio
dell'app (rilettura da disco) — lasciato all'utente.
