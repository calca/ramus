# Parsing `[[link]]` e `#tag`, con autocomplete

Stato: implementata. Verificato `open_page`/`list_pages` contro il vault
reale: front-matter scritto nel formato atteso
(`---\ntitle: ...\n---\n`), compatibile con Obsidian. Non verificabile
in questo sandbox: l'interazione del popup di autocomplete (digitare
`[[`, filtrare, navigare con le frecce, confermare) — serve un giro
manuale in `npm run tauri dev`.

## Motivazione

Primo pezzo di M2 (SPEC.md): "Parsing di `[[link]]` e `#tag`,
autocomplete durante la digitazione". Prerequisito per gli altri tre
pezzi di M2 (indice SQLite, pannello backlink, ricerca) — serve prima
che ci sia sintassi di link da indicizzare.

## Cosa NON cambia

Il formato su disco resta identico. `[[link]]` e `#tag` restano testo
letterale dentro `Block.content`, esattamente come digitati — non
diventano marks o nodi ProseMirror che richiederebbero
serializzazione/deserializzazione dedicata. Nessuna modifica a
`parser.rs`, `vault.rs`, `serializer.ts`, `deserializer.ts`. La sintassi
vive solo come **decorazione visiva** nell'editor, stessa tecnica già
usata per `block-focused` in `currentBlockHighlight.ts`: un plugin
ProseMirror che scansiona il testo con una regex e applica una classe
CSS, senza toccare il documento.

## Riconoscimento visivo

Nuovo `src/editor/linkTagHighlight.ts`, stesso pattern di
`currentBlockHighlight.ts` (`Extension` + `addProseMirrorPlugins` +
`props.decorations`), applicato a tutto il documento invece che al solo
blocco a fuoco:

- `[[link]]`: `/\[\[([^\]]+)\]\]/g` → classe `.editor-link`
- `#tag`: `/#[\w-]+/g` → classe `.editor-tag` (lettere, numeri, trattino,
  underscore; niente spazi, niente unicode esteso per ora)

CSS: colore `--ramus-sap` per entrambi. SPEC.md vieta esplicitamente
l'amber per link/tag/notifiche ("se perde l'esclusività smette di
indicare dove sei") — sap è l'accento neutro già usato per bullet e
guide di indentazione, coerente. Le parentesi quadre **restano
visibili** (nessun collasso stile "live preview" di Obsidian): sobrio,
coerente con l'assenza di marks WYSIWYG nel resto dell'editor.

## Autocomplete per `[[`

### Nuova dipendenza: `@tiptap/suggestion`

Motivazione: gestire in proprio trigger-detection, posizionamento del
popup, navigazione da tastiera e casi limite (composizione IME,
cancellazione, undo) è superficie ampia e fragile. `@tiptap/suggestion`
è l'utility ufficiale Tiptap per esattamente questo pattern (usata
internamente per le loro estensioni "mention"), stessa famiglia di
pacchetti già in uso (`@tiptap/core`, `@tiptap/pm`, `@tiptap/react`,
`@tiptap/starter-kit`, tutti 2.27.2). Verificato scaricando il pacchetto
(`npm pack @tiptap/suggestion@2.27.2`) e leggendone i tipi:
`peerDependencies` sono `@tiptap/core ^2.7.0` e `@tiptap/pm ^2.7.0`,
compatibili con quanto già installato.

Il trigger è la stringa `"[["` (non un singolo carattere): verificato
nel sorgente di `findSuggestionMatch` che `char` viene passato per
intero a `escapeForRegEx` e usato in un'unica regex — una stringa di
due caratteri funziona, non serve un carattere singolo.

### Front-matter: titolo leggibile per le pagine

Un blocco `---\ntitle: Nome\n---\n` in testa al file, **solo per
`pages/*.md`, mai per i journal** — stesso formato di Obsidian (SPEC.md,
principio 4: "Il formato su disco è compatibile con Obsidian"), non
un'invenzione nostra. Niente libreria YAML: parsing scritto a mano,
isolato in funzioni nuove che non toccano `parser.rs`:

```rust
/// Se `text` inizia con `---\n...\n---\n`, separa il blocco (delimitatori
/// inclusi) dal resto. `None` se non c'è o è malformato — in quel caso
/// tutto il testo va al corpo, trattato come prima (nessun crash).
fn split_front_matter(text: &str) -> (Option<&str>, &str) { ... }

/// Legge solo la riga `title: ...` da un front-matter grezzo. Altri
/// campi (se un file è stato toccato da Obsidian) restano ignorati ma
/// **non persi**: si preserva sempre il blocco raw, mai solo il titolo.
fn extract_title(front_matter: &str) -> Option<String> { ... }
```

`Page` guadagna `pub title: Option<String>` (`None` per i journal e per
pagine senza front-matter). **Non** un campo `front_matter` esposto al
frontend: il blob grezzo resta interamente lato core.

`read_page`/`write_page` (invariati nella firma pubblica) cambiano
dentro:
- `read_page`: separa il front-matter, estrae `title`, passa solo il
  corpo a `parser::parse` (che resta identico — zero rischio per il
  round-trip già testato dei blocchi).
- `write_page`: **prima di sovrascrivere**, rilegge il front-matter
  già presente nel file esistente (se c'è) e lo ripropone invariato
  davanti al nuovo corpo renderizzato con `parser::render` (anch'esso
  identico). Il command Tauri `write_page(path, blocks)` **non cambia
  firma**: il frontend continua a mandare solo `blocks`, non sa nulla
  del front-matter — altrimenti ogni battitura in una pagina
  cancellerebbe silenziosamente il suo titolo.

### Core: `Vault::list_pages` e `Vault::open_page`

```rust
pub struct PageSummary {
    pub slug: String,
    /// Titolo dal front-matter, o lo slug stesso se assente.
    pub title: String,
}

/// Pagine esistenti in pages/, slug + titolo, ordinate per titolo.
pub fn list_pages(&self) -> Result<Vec<PageSummary>, CoreError> { ... }

/// Apre (creando se non esiste) la pagina identificata da `name`: lo
/// slug del file è `slugify(name)`, e se il file va creato il
/// front-matter iniziale è `title: {name}` — il testo esatto digitato
/// dall'utente, non lo slug. Stesso pattern di `open_today`.
pub fn open_page(&self, name: &str) -> Result<Page, CoreError> {
    let relative_path = Self::page_relative_path(name);
    let abs = self.resolve(&relative_path)?;
    if !abs.exists() {
        let front_matter = format!("---\ntitle: {name}\n---\n");
        fs::write(&abs, format!("{front_matter}{}", parser::render(&[Block::new("")])))?;
    }
    self.read_page(&relative_path)
}
```

`page_relative_path`/`slugify` esistono già in `vault.rs` dal M1 ma
sono **inutilizzate**: nessuna UI ha mai scritto in `pages/` finora.
Questa spec è il primo uso reale.

### Command Tauri

```
list_pages() -> Result<Vec<PageSummary>, CoreError>
open_page(name: String) -> Result<Page, CoreError>
```

Wrapper sottili, stesso schema di `open_today`/`read_page`.

### Comportamento del popup

- Digitando `[[`, si apre un popup con le pagine esistenti il cui
  **titolo** combacia col testo digitato (filtro su `title`, non sullo
  slug — è quello che l'utente legge e digita).
- Se la query non combacia esattamente nessuna pagina esistente, in
  fondo alla lista compare **"Crea «{query}»"**.
- Selezionando una pagina esistente: si inserisce `[[title]]` (il
  titolo, testo leggibile) al posto del testo digitato — non lo slug.
- Selezionando "Crea «{query}»": si inserisce `[[{query}]]` (il testo
  esatto digitato) e si chiama `open_page(query)` — il file
  `pages/slugify(query).md` viene creato subito con
  `title: {query}` nel front-matter. Non si apre nessuna vista della
  pagina (non esiste ancora, vedi "Fuori scope") — solo il file viene
  creato.
- Frecce su/giù per navigare, Invio per confermare, Escape per
  annullare: gestito dai callback `onKeyDown`/`onStart`/`onUpdate`/
  `onExit` di `render()`, popup montato come un piccolo componente
  React (`LinkSuggestionList`) dentro un container DOM creato/distrutto
  nei callback — stesso pattern dell'esempio ufficiale "mention" di
  Tiptap adattato a React.

### Come si risolve un `[[link]]` a un file

Sempre `slugify(testoFraParentesi)` → `pages/{slug}.md`, mai al
contrario. Funziona perché `open_page`/il flusso "Crea «query»" sopra
garantiscono che slug e titolo siano sempre coerenti fra loro alla
creazione. Stessa funzione `slugify` usata per il filtro dell'autocomplete
e (nella prossima spec, sulla navigazione) per risolvere un click.
Collisioni tipo "Progetto X" / "progetto x" → stesso slug `progetto-x`:
comportamento preesistente di `slugify` dal M1, non introdotto né
risolto qui (vedi "Fuori scope").

### Limite noto e accettato

Un `[[link]]` digitato manualmente per intero, senza mai passare dal
popup (es. incollato, o scritto molto rapidamente ignorando i
suggerimenti), **non** crea automaticamente la pagina — la creazione è
legata solo all'azione esplicita di conferma nel popup, non a una
scansione passiva del testo (le decorazioni restano pure, senza effetti
collaterali). La pagina si materializzerà solo quando esisterà un modo
di aprirla (fuori scope qui).

## `#tag`: solo riconoscimento, nessun autocomplete

Deciso esplicitamente: niente lista di suggerimenti per ora. Per
proporre tag già usati servirebbe scansionare tutto il vault (contenuto
dei blocchi, non solo nomi di file) — è esattamente il lavoro per cui
è previsto l'indice SQLite, prossimo pezzo di M2. Farlo qui vorrebbe
dire duplicare quel lavoro con una scansione ad-hoc, oppure spedire un
autocomplete basato solo sulle sezioni journal già caricate in memoria
(incompleto, ordine arbitrario). Meglio aspettare l'indice: il
riconoscimento visivo di `#tag` (colorazione) resta comunque, solo il
popup di suggerimenti arriva dopo.

## Fuori scope per questa spec

- Click su un `[[link]]` per navigare alla pagina: non esiste ancora
  una vista pagina (solo la vista journal verticale) — spec separata,
  vedi `specs/2026-09-02-navigazione-pagine.md`.
- Autocomplete dei tag (vedi sopra): arriva con l'indice SQLite.
- Rinominare una pagina (cambiare il `title` di una pagina esistente):
  nessuna UI per farlo — il titolo si imposta solo alla creazione.
- Link a date di journal in stile `[[2026-09-02]]`: questa spec copre
  solo pagine in `pages/`. Se serve linkare i journal, è un'estensione
  piccola (seconda fonte di candidati da `list_journals`, già esistente)
  ma va decisa esplicitamente, non assunta.
- Gestione di collisioni di slug (es. "Progetto X" e "progetto x"
  producono lo stesso slug `progetto-x`): comportamento preesistente di
  `slugify` dal M1, non introdotto né risolto qui.

## Test da scrivere (core)

- `split_front_matter`/`extract_title`: round-trip su testo con e
  senza front-matter, front-matter malformato non manda in panico
  (tutto trattato come corpo).
- `open_page` crea il file con front-matter `title: {name}` e un
  blocco vuoto se non esiste.
- `open_page` su una pagina già esistente non sovrascrive il contenuto
  né il front-matter (stesso comportamento di `open_today`).
- `write_page` su una pagina con front-matter esistente lo preserva
  intatto dopo un salvataggio che cambia solo i blocchi.
- `write_page` su un journal (mai front-matter) si comporta esattamente
  come prima — nessuna regressione sul path M1.
- `list_pages` elenca slug+titolo, ordinati per titolo; titolo assente
  → ricade sullo slug.
- `list_pages` su `pages/` vuota o assente ritorna lista vuota, non
  errore (stesso pattern di `stats`/`list_journals`).

## Verifica

`cargo test`, `npm run typecheck` per la parte automatizzabile. Il
resto (aprire il popup digitando `[[`, filtrare, selezionare, la
creazione del file) è interazione da tastiera in tempo reale — non
verificabile in questo sandbox, serve un giro manuale in
`npm run tauri dev`.
