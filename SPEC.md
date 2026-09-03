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
- **git2** — sync Git (dalla milestone 3)
- **rmcp** — server MCP, solo in `crates/ramus-mcp` (dalla milestone 5)

Dipendenze Rust iniziali: `serde`, `serde_json`, `notify`, `pulldown-cmark`,
`thiserror`, `dirs`. Aggiunte in M2: `rusqlite`, `tantivy`. Aggiunta in M3:
`git2` (scelta su `gitoxide` — verificato, `gix` non espone ancora un push
funzionante nel crate principale). Aggiunta in M5, solo nel nuovo crate
`ramus-mcp` (mai in `ramus-core`, vedi CLAUDE.md regola 1): `rmcp`, SDK
Rust ufficiale del Model Context Protocol, più `schemars` (versione
allineata a quella usata internamente da `rmcp`, necessaria perché la
derive `#[derive(JsonSchema)]` richiede lo stesso crate in scope).
`dirs` spostata fuori da `ramus-core` in M6 (Android non è coperto in
modo affidabile, verificato): resta solo in `src-tauri` e `ramus-mcp`,
per il calcolo dei path lato desktop.

## Struttura del repository

```
ramus/
├── src/                    # frontend React
│   ├── components/
│   ├── editor/             # Tiptap: estensioni, serializer, deserializer
│   ├── lib/                # wrapper tipizzati sui command Tauri
│   └── App.tsx
├── crates/
│   ├── ramus-core/       # crate puro Rust, nessuna dipendenza da Tauri
│   │   ├── src/
│   │   │   ├── vault.rs    # gestione cartella, path, creazione file
│   │   │   ├── block.rs    # modello a blocchi
│   │   │   ├── parser.rs   # markdown <-> albero di blocchi
│   │   │   └── config.rs
│   │   └── Cargo.toml
│   └── ramus-mcp/        # binario server MCP (M5), usa ramus-core
│       ├── src/main.rs
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
- Apertura automatica del journal di oggi — resta vero anche a cavallo
  di mezzanotte se l'app è rimasta aperta, vedi
  `specs/M1/2026-09-02-nuovo-giorno-automatico.DONE.md`
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

**M3 — Git** (completa)
- Commit automatico su intervallo configurabile — nessun `git init`
  forzato, l'utente attiva la sync esplicitamente da Impostazioni;
  vedi `specs/M3/2026-09-02-sync-git-locale.DONE.md`
- Pull all'avvio, push a intervallo — autenticazione delegata
  interamente al sistema (SSH agent, credential helper), nessun account
  Ramus; vedi `specs/M3/2026-09-02-sync-git-remoto.DONE.md`
- Stato della sync visibile nella UI (badge nell'header finché M4 non
  introduce la status bar); in caso di conflitto, stop e avviso
  esplicito, mai merge automatico silenzioso

**M4 — UI** (completa)
- Rifinitura dell'interfaccia raccolta durante l'uso reale dell'app, non
  legata a una singola feature.
- Header ridotto a logo + titolo + 3 icone (comprimi, comandi,
  impostazioni); navigazione del journal e badge di stato sync spostati
  in una status bar sempre presente in fondo alla finestra, nascosta in
  modalità compatta — vedi `specs/M4/2026-09-02-header-status-bar.DONE.md`.
- Command palette: evoluzione della ricerca full-text (M2) con azioni
  dell'app, pagine aperte di recente (persistite per vault) e
  creazione pagine dalla stessa lista — vedi
  `specs/M4/2026-09-02-command-palette.DONE.md`.
- Scorciatoie app-level configurabili in un registro unico (non più un
  solo campo `search_shortcut`), più una cheatsheet (`Mod+/`) con le
  scorciatoie app e quelle fisse dell'editor — vedi
  `specs/M4/2026-09-02-scorciatoie-configurabili.DONE.md`.
- Riordino blocchi fratelli da tastiera (Alt+Su/Giù, sottoalbero
  incluso) — scorciatoia fissa dell'editor, non nel registro
  configurabile — vedi
  `specs/M4/2026-09-02-riordino-blocchi-tastiera.DONE.md`.
- Focus mode (`Mod+.`, nasconde header e status bar) e navigazione fra
  giorni del journal da tastiera (`Mod+↑`/`Mod+↓`) — vedi
  `specs/M4/2026-09-02-focus-mode-navigazione-giorni.DONE.md`.
- Task nei blocchi (`- [ ] `/`- [x] `, sintassi Obsidian/GFM): click
  sul marker per il toggle, `Mod-Enter` per il ciclo a tre stati
  (normale → da fare → fatto), e spostamento automatico a oggi dei
  task non fatti rimasti negli ultimi N giorni (configurabile) al
  cambio di giorno — vedi
  `specs/M4/2026-09-02-task-todo-done.DONE.md`.

**M5 — AI** (completa)
- Non un'AI integrata nell'app: un server MCP (`ramus-mcp`, nuovo
  crate binario, indipendente dalla GUI) che espone il vault a un
  agente/tool AI già scelto e configurato dall'utente (Claude Code,
  Claude Desktop, o qualunque client MCP) — vincolo "nessun invio
  automatico" soddisfatto perché Ramus non contatta mai un'AI, è
  l'agente a interrogare Ramus. Strumenti di sola lettura (ricerca,
  lettura pagine/journal, backlink, tag) — vedi
  `specs/M5/2026-09-02-mcp-server-lettura.DONE.md`, che include anche
  `ramus-mcp --print-config` per generare lo snippet di configurazione
  del client MCP.
- Strumenti di scrittura (`write_page`, `open_today`, `open_page`),
  stesso crate — flag `--read-only` per escluderli del tutto
  dall'elenco esposto (scrittura abilitata di default) — vedi
  `specs/M5/2026-09-02-mcp-server-scrittura.DONE.md`.
- Sezione "MCP" in Impostazioni: interruttore di attivazione (un vero
  kill switch, letto da `ramus-mcp` all'avvio dallo stesso
  `config.json` — non solo uno stato mostrato in GUI) e, se attivo e
  il binario compilato viene trovato accanto a quello dell'app, lo
  snippet di configurazione già pronto da incollare nel client — vedi
  `specs/M5/2026-09-03-mcp-impostazioni.DONE.md`.

**M6 — Mobile (Android/iOS)**
- Supporto mobile nativo di Tauri v2 (stesso frontend, stesso
  `ramus-core`), non un client FFI separato. Percorsi vault/config
  spostati da `dirs` al resolver di Tauri (Android non è coperto in
  modo affidabile da `dirs`, verificato): `Config::load_or_init` non
  li calcola più da sé, li riceve iniettati dal chiamante — `dirs` su
  desktop (invariato), `app.path()` su mobile (`app_data_dir()` per il
  vault, `app_config_dir()` per `config.json`) — vedi
  `specs/M6/2026-09-03-supporto-mobile-fondamenta.DONE.md`. Il ramo
  mobile è scritto ma non compilato/verificato in questo sandbox
  (nessun target Android/iOS installato); il resto della spec
  (`tauri android/ios init`, build reale) resta da fare. Nessun
  selettore di cartella su mobile (non esiste nell'API), il vault vive
  in un percorso fisso. Impatti su M1-M5 catalogati in
  `specs/M6/2026-09-03-impatti-milestone-precedenti.TODO.md` — M3
  (credenziali, sync in background) e M4 (scorciatoie da tastiera)
  richiedono adattamenti sostanziali quando ci si arriva, M5 resta
  desktop-only per natura.

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
