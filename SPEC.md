# Ramus — Spec di progetto

App desktop per journaling e note, outliner a blocchi su file markdown locali.
Ispirata a Logseq, ma volutamente più piccola: niente query language, niente
plugin system, niente sync proprietaria.

## Principi non negoziabili

1. **I file markdown sono la source of truth.** Ogni altra struttura dati
   (indice, cache, database) è derivata e rigenerabile cancellandola.
2. **Il core in Rust non dipende da Tauri.** Deve poter essere compilato e
   testato da solo, ed esposto in futuro via FFI a un client mobile.
3. **Zero attrito all'avvio.** L'app si apre direttamente sul journal di oggi.
   Nessuna schermata di selezione, nessun wizard, nessun login.
4. **Il formato su disco è compatibile con Obsidian.** Le stesse note devono
   restare apribili e modificabili da altri editor markdown.

## Stack

- **Tauri v2** — guscio desktop (macOS, Windows, Linux)
- **Rust** — core: parsing, indice, filesystem, git
- **React + TypeScript + Vite** — UI
- **Tiptap** (ProseMirror) — editor outliner
- **SQLite** via `rusqlite` — indice derivato (dalla milestone 2)
- **tantivy** — ricerca full-text (dalla milestone 2)

Dipendenze Rust iniziali: `serde`, `serde_json`, `notify`, `pulldown-cmark`,
`thiserror`, `dirs`. Aggiunte in M2: `rusqlite`, `tantivy`.
Da aggiungere più avanti, **non ora**: `git2` o `gitoxide` (M3).

## Struttura del repository

```
ramus/
├── src/                    # frontend React
│   ├── components/
│   ├── editor/             # Tiptap: estensioni, serializer, deserializer
│   ├── lib/                # wrapper tipizzati sui command Tauri
│   └── App.tsx
├── crates/
│   └── ramus-core/       # crate puro Rust, nessuna dipendenza da Tauri
│       ├── src/
│       │   ├── vault.rs    # gestione cartella, path, creazione file
│       │   ├── block.rs    # modello a blocchi
│       │   ├── parser.rs   # markdown <-> albero di blocchi
│       │   └── config.rs
│       └── Cargo.toml
└── src-tauri/              # guscio: solo command che chiamano ramus-core
    ├── src/
    │   ├── main.rs
    │   └── commands.rs
    └── Cargo.toml
```

Cargo workspace alla radice, membri: `crates/ramus-core` e `src-tauri`.

## Formato su disco

Cartella di lavoro (default `~/Journal`, configurabile):

```
<vault>/
├── journals/
│   └── 2026-09-01.md       # ISO 8601, una pagina per giorno
└── pages/
    └── nome-pagina.md      # slug lowercase, spazi -> trattini
```

Regole di formato, da rispettare rigidamente perché condizionano il merge git:

- Un blocco = **una riga**. Mai wrappare, mai spezzare un blocco su più righe.
- Ogni blocco inizia con `- ` dopo l'indentazione.
- L'annidamento usa **due spazi** per livello (compatibile con Obsidian).
- Il file termina sempre con un newline.
- Nessun front-matter nella milestone 1.
- Nessun ID sui blocchi nella milestone 1. Verranno aggiunti solo quando
  serviranno le block reference, come proprietà a fine riga.

Esempio:

```markdown
- Riunione con il cliente
  - Deciso di rimandare il rilascio
  - Da verificare: capacità del team a ottobre
- Nota personale
```

## Modello dati (core)

```rust
pub struct Block {
    pub content: String,      // testo markdown inline, senza il "- "
    pub children: Vec<Block>,
}

pub struct Page {
    pub path: PathBuf,
    pub blocks: Vec<Block>,
}
```

Il parser deve garantire il **round-trip esatto**: `parse(render(page)) == page`
e, per file già conformi al formato, `render(parse(text)) == text`.
Questo è il primo test da scrivere, con property test se possibile.

## Command Tauri (milestone 1)

```
get_config() -> Config                    // include vault_path
set_vault_path(path: String) -> Config
open_today() -> Page                      // crea il file se non esiste
read_page(path: String) -> Page
write_page(path: String, blocks: Vec<Block>) -> ()
```

Il frontend non conosce mai i percorsi assoluti né tocca il filesystem
direttamente: passa solo path relativi al vault.

## Editor (la parte critica)

Tiptap con `StarterKit`, tenendo `bulletList` e `listItem`, disattivando
heading, blockquote, codeBlock e horizontalRule per la milestone 1.

- **Tab** → `sinkListItem('listItem')`
- **Shift+Tab** → `liftListItem('listItem')`
- **Enter** → comportamento default (nuovo item allo stesso livello)
- **Backspace** su item vuoto → lift, poi merge

Serializzazione: scrivere **funzioni proprie** che convertono fra il JSON del
documento ProseMirror e `Block[]`. Non usare estensioni markdown di terze parti:
il round-trip che offrono normalizza la formattazione e rompe i diff git.
Sono circa 40 righe ricorsive per direzione.

Salvataggio: debounce di 500 ms dall'ultima modifica, più flush immediato
su blur della finestra e su chiusura dell'app.

## File watcher

`notify` osserva il vault. Se un file cambia sul disco mentre è aperto e
**non** ci sono modifiche locali non salvate, ricarica silenziosamente.
Se ci sono modifiche pendenti, mostra un avviso e non sovrascrivere niente.
La risoluzione dei conflitti non è in scope: serve solo non perdere dati.

## Identità visiva

Nome: **Ramus**. Il marchio è un albero di blocchi — nodi collegati da guide
di indentazione — che è la stessa grammatica visiva dell'editor.

Asset in `assets/`:

| File | Uso |
| --- | --- |
| `logo.svg` | marchio pieno, sopra i 32px |
| `favicon.svg` | versione a tre nodi, tratto più spesso, per 16-32px |
| `logo-mono.svg` | usa `currentColor`, eredita il colore dal contesto |
| `mascotte.svg` | Stecco, la mascotte — solo sopra i 64px |
| `palette.css` | variabili colore, con varianti dark |

**Non scalare `logo.svg` sotto i 32px**: usare `favicon.svg`, che è
semplificato apposta. Sono due file distinti, non due dimensioni dello stesso.

### Mascotte

Stecco, un insetto stecco. Va usato **solo** in schermate vuote, stati di
errore, onboarding, README e changelog. Mai dentro l'area di scrittura:
l'editor resta sobrio. Sotto i 64px non è leggibile — per gli spazi piccoli
si usa il marchio.

### Palette

| Token | Light | Dark | Uso |
| --- | --- | --- | --- |
| `--ramus-ink` | `#1C1A17` | `#F5F1E8` | testo dei blocchi |
| `--ramus-sap` | `#2F6B4F` | `#4E9B77` | bullet, guide di indentazione, sync ok |
| `--ramus-amber` | `#C98A2E` | `#E0A745` | **solo** giorno corrente e blocco in focus |
| `--ramus-stone` | `#8A857C` | `#9C978D` | metadati, testo secondario, placeholder |
| `--ramus-paper` | `#F5F1E8` | `#16150F` | sfondo |

Regola: due soli accenti, tutto il resto neutro. L'amber non va usato per link,
tag o notifiche — se perde l'esclusività smette di indicare "dove sei".
I colori si consumano solo tramite le variabili CSS, mai hex inline.

### Icone applicative

Generate con `npm run tauri icon assets/logo.svg`. Prima di lanciare il comando,
aggiungere un margine del 12% attorno al segno: la maschera di macOS taglia
il marchio contro il bordo.

## Milestone

**M1 — Journal funzionante** (completa)
- Creazione vault di default al primo avvio, senza chiedere nulla
- Apertura automatica del journal di oggi
- Outliner con indent/outdent e navigazione da tastiera
- Persistenza su file con debounce
- File watcher con ricarica sicura
- Vista journal verticale stile Logseq: oggi in cima, i giorni
  precedenti sotto in scroll infinito, più salto a data arbitraria —
  sostituisce la navigazione a giorno singolo (precedente/successivo)
  originariamente prevista qui, vedi `specs/M1/2026-09-02-journal-vista-verticale.DONE.md`

**M2 — Link e ricerca** (completa)
- Parsing di `[[link]]` e `#tag`, autocomplete durante la digitazione —
  autocomplete dei tag arrivato dopo l'indice SQLite (serviva
  `list_tags`), vedi `specs/M2/2026-09-02-autocomplete-tag.DONE.md`
- Indice SQLite rigenerabile con pagine, link e blocchi
- Pannello backlink sulla pagina aperta — backlink da un journal
  mostrati come testo non cliccabile, non esiste ancora una vista per
  un singolo giorno isolato dal journal verticale, vedi
  `specs/M2/2026-09-02-pannello-backlink.DONE.md`
- Ricerca full-text con `tantivy` — granularità per pagina/giorno
  intero (non per blocco), scorciatoia configurabile in Impostazioni,
  vedi `specs/M2/2026-09-02-ricerca-full-text.DONE.md`

**M3 — Git**
- Commit automatico su intervallo configurabile
- Pull all'avvio, push a intervallo
- Stato della sync visibile nella UI; in caso di conflitto, stop e avviso
  esplicito, mai merge automatico silenzioso

**M4 — UI**
- Da specificare più avanti. Rifinitura dell'interfaccia raccolta durante
  l'uso reale dell'app, non legata a una singola feature — header più
  compatto, status bar in basso, idee simili che emergono nel tempo.

**M5 — AI**
- Da specificare più avanti. Vincolo già deciso: nessun invio automatico di
  contenuti a servizi esterni. Ogni chiamata deve essere esplicita per blocco
  o per pagina, con indicazione chiara di cosa viene inviato.

## Fuori scope (non implementare, nemmeno "in preparazione")

- Multi-vault e switcher fra vault
- Block reference e embed
- Query language
- Plugin system
- Sync proprietaria o account utente
- Editor WYSIWYG per tabelle, canvas, grafi
- Client mobile (il vincolo di riuso è già garantito da `ramus-core`)

## Convenzioni

- Errori Rust con `thiserror`, mai `unwrap()` nei command
- Test unitari sul parser e sul serializer prima di collegare la UI
- Commit convenzionali (`feat:`, `fix:`, `refactor:`)
- Nessuna dipendenza nuova senza motivo scritto nel commit
