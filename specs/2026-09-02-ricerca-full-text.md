# Ricerca full-text con tantivy

Stato: implementata. Domande aperte risolte così: (1) riuso del diff
di `Index::sync` — confermato, `SyncOutcome` guida `SearchIndex` senza
contabilità di mtime propria; (2) scorciatoia Cmd/Ctrl+K **più
configurabile in Impostazioni** (estensione non prevista nella proposta
originale — nuovo campo `Config::search_shortcut`, cattura da tastiera
in `SettingsPanel`, vedi `src/lib/shortcut.ts`); (3) snippet con
evidenziazione HTML — confermato; (4) granularità per pagina/giorno —
confermata.

## Motivazione

Quarto e ultimo pezzo di M2 (SPEC.md): "Ricerca full-text con
`tantivy`". `tantivy` non è ancora una dipendenza del progetto (a
differenza di `rusqlite`, non era pre-approvata nello Stack di
SPEC.md sotto un nome esplicito, ma la ricerca full-text con tantivy
è nominata testualmente nella milestone stessa — la dipendenza è
implicita nel requisito). Verificato: `tantivy 0.26.1` su crates.io,
richiede `rust-version 1.86` (il toolchain locale è 1.96, compatibile).

## Perché un indice separato da quello SQLite

`specs/2026-09-02-indice-sqlite.md` aveva messo esplicitamente "ricerca
full-text (tantivy, indice separato)" fuori scope. SQLite (via `LIKE`
o anche FTS5) potrebbe bastare per un vault piccolo, ma tantivy dà
tokenizzazione, ranking per rilevanza e snippet con evidenziazione
"gratis" — è la scelta già nominata in SPEC.md, non va rimessa in
discussione qui.

## Granularità: per pagina/giorno, non per blocco

L'indice SQLite indicizza per blocco (serve per i link, dove la
sorgente esatta conta). La ricerca full-text qui indicizza per
**pagina intera** (un giorno di journal, o una pagina): il contenuto
indicizzato è tutti i blocchi di una pagina concatenati. Motivo: non
esiste (né questa spec la introduce) una vista "singolo blocco" da
aprire — il risultato di una ricerca porta sempre e solo ad aprire una
pagina o saltare a un giorno, mai a un blocco isolato. Indicizzare per
blocco aggiungerebbe complessità (schema, aggregazione dei risultati
per pagina in fase di query) senza un consumatore.

## Schema tantivy

```rust
let mut schema_builder = Schema::builder();
let path = schema_builder.add_text_field("path", STRING | STORED);   // match esatto, per delete_term
let kind = schema_builder.add_text_field("kind", STRING | STORED);   // "page" | "journal"
let title = schema_builder.add_text_field("title", TEXT | STORED);   // tokenizzato + cercabile
let content = schema_builder.add_text_field("content", TEXT | STORED); // blocchi concatenati con "\n"
let schema = schema_builder.build();
```

`STRING` (non tokenizzato) per `path`/`kind`: serve un match esatto per
`delete_term` quando una pagina viene reindicizzata o rimossa, mai una
ricerca full-text su questi campi. `STORED` su tutti i campi: servono
per ricostruire il risultato (path/kind/title) e per generare lo
snippet dal testo originale di `content` senza una query aggiuntiva al
vault.

## Dove vive il file

`<vault>/.ramus/search-index/` — cartella (tantivy richiede una
directory, non un singolo file), accanto a `.ramus/index.sqlite3`.
Stesso principio di invisibilità/rigenerabilità di SPEC.md #1: se la
cartella viene cancellata, si ricostruisce da zero al prossimo avvio.

## Nuovo modulo: `ramus-core/src/search.rs`

```rust
pub struct SearchHit {
    pub path: String,
    pub kind: String,           // "page" | "journal"
    pub title: Option<String>,
    /// HTML generato da tantivy (`Snippet::to_html()`): testo
    /// circostante fuggito, termini che combaciano avvolti in `<b>`.
    /// Sicuro da rendere con `dangerouslySetInnerHTML` — l'unico
    /// markup iniettato è quello che genera tantivy stesso, il resto
    /// del testo passa da `encode_minimal` (fuga HTML) prima di
    /// arrivare qui.
    pub snippet_html: String,
}

pub struct SearchIndex {
    index: tantivy::Index,
    reader: tantivy::IndexReader,
    path_field: Field,
    kind_field: Field,
    title_field: Field,
    content_field: Field,
}

impl SearchIndex {
    pub fn open(vault_root: &Path) -> Result<Self, CoreError> { ... }

    /// Rilegge una pagina dal vault e sostituisce il suo documento
    /// nell'indice (delete_term su `path` + reinserimento + commit).
    /// Stessa idea di `Index::refresh_page` (SQLite), stessa "unità di
    /// lavoro": una pagina alla volta, non un batch.
    pub fn refresh_page(&self, vault: &Vault, relative_path: &str) -> Result<(), CoreError> { ... }

    /// Rimuove il documento di una pagina non più presente su disco.
    pub fn remove_page(&self, relative_path: &str) -> Result<(), CoreError> { ... }

    /// Cerca su `title` + `content`, fino a `MAX_SEARCH_RESULTS`
    /// risultati ordinati per rilevanza. Query vuota o non
    /// interpretabile (es. virgolette sbilanciate) → lista vuota, non
    /// errore: l'utente sta ancora digitando, non è un caso da trattare
    /// come fallimento.
    pub fn search(&self, query: &str) -> Result<Vec<SearchHit>, CoreError> { ... }
}

const MAX_SEARCH_RESULTS: usize = 20;
```

`refresh_page` legge la pagina, appiattisce i blocchi (**stessa**
funzione `flatten_blocks` già in `index.rs`, resa `pub(crate)` e
riusata invece di duplicata) e unisce i `content` con `"\n"`.

### Errore nuovo: `CoreError::Search`

```rust
#[error("errore di ricerca: {0}")]
Search(#[from] tantivy::TantivyError),
```

Stesso schema già usato per `CoreError::Index(#[from] rusqlite::Error)`
nell'indice SQLite.

### Reader e freschezza dei risultati

`Index::reader()` di default usa `ReloadPolicy::OnCommitWithDelay`
(ricarica il reader dopo un commit, ma con un ritardo in background —
non garantito immediato). Per un'app mono-utente a bassa frequenza di
ricerca, la scelta più semplice e corretta è **non fidarsi del
delay**: il reader si costruisce con `ReloadPolicy::Manual`
(`index.reader_builder().reload_policy(ReloadPolicy::Manual).try_into()?`)
e si chiama `reader.reload()?` esplicitamente all'inizio di ogni
`search()`, prima di ottenere il `Searcher`. Costo: un piccolo overhead
per ogni ricerca (non per ogni tasto, vedi debounce lato frontend) in
cambio della garanzia che i risultati riflettano sempre l'ultimo
salvataggio, anche subito dopo — nessuna finestra di risultati stantii
da spiegare o da far scoprire all'utente.

### Snippet

```rust
let searcher = self.reader.searcher();
let query_parser = QueryParser::for_index(&self.index, vec![self.title_field, self.content_field]);
let Ok(parsed) = query_parser.parse_query(query) else {
    return Ok(Vec::new());
};
let top_docs = searcher.search(&parsed, &TopDocs::with_limit(MAX_SEARCH_RESULTS))?;

let mut snippet_gen = SnippetGenerator::create(&searcher, &parsed, self.content_field)?;
snippet_gen.set_max_num_chars(160);

let mut hits = Vec::new();
for (_score, doc_address) in top_docs {
    let doc: TantivyDocument = searcher.doc(doc_address)?;
    let snippet = snippet_gen.snippet_from_doc(&doc);
    hits.push(SearchHit {
        path: /* doc.get_first(path_field) */,
        kind: /* ... */,
        title: /* ... */,
        snippet_html: snippet.to_html(),
    });
}
Ok(hits)
```

Se `content_field` di un documento non contiene nessun termine della
query (es. il match è avvenuto solo su `title`), `Snippet::to_html()`
produce un frammento vuoto: la UI mostra comunque il titolo, coerente
con "un match sul titolo è un match valido anche senza estratto".

## Sincronizzazione: riuso del diff già calcolato da `Index::sync`

Per evitare che `SearchIndex` debba tenere una propria contabilità di
mtime (duplicando la logica già scritta per l'indice SQLite in
`specs/2026-09-02-indice-sqlite.md`), **`Index::sync` cambia firma**
per restituire cosa ha effettivamente cambiato:

```rust
pub struct SyncOutcome {
    pub refreshed: Vec<String>,
    pub removed: Vec<String>,
}

pub fn sync(&self, vault: &Vault) -> Result<SyncOutcome, CoreError> { ... }
```

(prima: `Result<(), CoreError>` — il chiamante attuale, in
`src-tauri/src/lib.rs`/`commands.rs`, oggi scarta il risultato con `?`
seguito da niente: cambia in `let outcome = index.sync(&vault)?;`,
modifica meccanica, nessun impatto sul formato su disco).

Il livello command orchestra i due indici usando lo stesso risultato:

```rust
let outcome = index.sync(&vault)?;
for path in &outcome.refreshed {
    search_index.refresh_page(&vault, path)?;
}
for path in &outcome.removed {
    search_index.remove_page(path)?;
}
```

Stesso schema per gli aggiornamenti incrementali durante la sessione:
ovunque oggi si chiama `index.refresh_page(&vault, &path)?` (in
`write_page`, `open_today`, `open_page`, e nel callback del watcher per
modifiche esterne), si aggiunge subito dopo
`search_index.refresh_page(&vault, &path)?`. Nessuna nuova logica di
staleness: `SearchIndex` è "dumb", esegue solo quello che il livello
command gli dice di fare, guidato dallo stesso `Index::sync` che già
governa l'indice SQLite.

`AppState` guadagna `pub search_index: Mutex<SearchIndex>`, stesso
pattern di `index: Mutex<Index>` — creato in `setup()` e in
`set_vault_path` subito dopo l'`Index` corrispondente.

## Command Tauri

```rust
#[tauri::command]
pub fn search(query: String, state: State<AppState>) -> Result<Vec<SearchHit>, CoreError> {
    lock_search_index(&state)?.search(&query)
}
```

## Frontend

### `src/lib/types.ts` / `src/lib/commands.ts`

```ts
export interface SearchHit {
  path: string;
  kind: "page" | "journal";
  title: string | null;
  snippet_html: string;
}

export function search(query: string): Promise<SearchHit[]> {
  return invoke<SearchHit[]>("search", { query });
}
```

### `src/components/SearchPanel.tsx` (nuovo)

Quarto pannello modale, stesso schema di `SettingsPanel`/`AboutPanel`
(`.settings-backdrop` riusato) — `App.tsx` estende
`activePanel: "settings" | "about" | "search" | null`.

- Input di testo, focus automatico all'apertura.
- Debounce 250ms (più corto dei 500ms del salvataggio: la ricerca è
  locale e già rapida, un ritardo lungo si sentirebbe "lento" mentre si
  digita) prima di chiamare `search(query)`.
- Query vuota → nessuna chiamata, lista risultati vuota, nessun
  placeholder "digita per cercare" (coerente con la sobrietà generale).
- Ogni risultato: badge/etichetta piccola per `kind`, titolo (`title ??`
  data estratta da `path` se `kind === "journal"`), snippet via
  `dangerouslySetInnerHTML={{ __html: hit.snippet_html }}`.
- Click su un risultato → `onSelect(hit)`, chiude il pannello.

### Apertura del pannello

Bottone nell'header (`.app-header`), accanto all'ingranaggio
impostazioni — icona lente 🔍, stesso stile di `.settings-button`.
Nascosto in modalità compatta, stesso trattamento già riservato a
`.settings-button:not(.compact-toggle)`.

### Scorciatoia configurabile (estensione decisa in fase di conferma)

Non solo Cmd/Ctrl+K fisso: `Config` guadagna
`pub search_shortcut: String` (default `"Mod+K"`, stesso trattamento
di `theme` per compatibilità con `config.json` precedenti), con
`Config::set_search_shortcut` e il command `set_search_shortcut`.

`src/lib/shortcut.ts` (nuovo): formato canonico `"Mod+K"` /
`"Mod+Shift+F"` — "Mod" è il modificatore primario della piattaforma
(Cmd su macOS, Ctrl altrove) ed è **sempre obbligatorio**: senza,
qualunque lettera digitata normalmente nell'editor aprirebbe il
pannello, rompendo la scrittura. `normalizeShortcut(event)` cattura un
`KeyboardEvent` in questa forma (`null` se manca il modificatore
primario); `matchesShortcut(event, shortcut)` confronta un evento
contro lo shortcut salvato; `formatShortcut(shortcut)` lo rende
leggibile per la UI (`⌘K` su macOS, `Ctrl+K` altrove).

`SettingsPanel` guadagna una sezione "Ricerca": un bottone che mostra
la scorciatoia corrente e, al click, entra in modalità "registrazione"
— il prossimo keydown valido (con modificatore primario) viene
catturato e salvato. Il listener di cattura è in fase `capture` con
`stopPropagation()`: durante la registrazione, Escape annulla solo la
registrazione invece di chiudere anche l'intero pannello (altrimenti
il listener Escape di `Modal`, in bubble su `window`, lo vedrebbe
comunque).

`App.tsx`: un `useEffect` con un listener `keydown` globale confronta
ogni evento con `config.search_shortcut` via `matchesShortcut`.

### Selezione di un risultato

```ts
const handleSearchSelect = useCallback(
  async (hit: SearchHit) => {
    setActivePanel(null);
    if (hit.kind === "page") {
      await navigateToPage(hit.title ?? /* slug da hit.path */);
    } else {
      if (view.kind === "page") {
        await returnToJournal();
      }
      const date = journalDateFromPath(hit.path);
      await jumpToDate(date);
    }
  },
  [navigateToPage, returnToJournal, jumpToDate, view],
);
```

Riusa `jumpToDate`, già esistente in `App.tsx` per "salta a data" nella
`JournalControls` — stessa funzione, stesso comportamento (trova il
giorno più vicino `<= target` e ci scrolla): qui `target` è sempre un
giorno che esiste per certo (viene dall'indice), quindi il match è
sempre esatto.

## Fuori scope per questa spec

- Sintassi di ricerca avanzata visibile in UI (operatori `AND`/`OR`,
  frasi tra virgolette): il `QueryParser` di tantivy li supporta già
  nativamente, ma non c'è nessun suggerimento/hint per l'utente su come
  usarli — restano "scoperti" finché non serve documentarli.
- Ricerca fuzzy/tollerante a errori di battitura: comportamento di
  default di tantivy (match esatto sui token), nessuna configurazione
  aggiuntiva in questa spec.
- Filtri (solo pagine, solo journal, intervallo di date): un solo campo
  di ricerca, nessun filtro.
- Evidenziazione dei match anche nel `title`, non solo nel `content`:
  `SnippetGenerator` qui genera lo snippet solo dal campo `content` —
  un match sul titolo resta visibile perché il titolo è comunque
  mostrato per intero, solo senza `<b>`.
- Indicizzazione incrementale "vera" lato tantivy (mtime propria):
  deciso di riusare il diff di `Index::sync` (vedi sopra) invece di
  duplicare la logica — se in futuro i due indici devono divergere
  (es. l'uno serve ma non l'altro), si riconsidera.

## Domande aperte

1. **Riuso del diff di `Index::sync` (cambio di firma)**: l'alternativa
   è dare a `SearchIndex` una propria logica di sync indipendente
   (proprio `list_disk_pages` + proprio confronto mtime, letto dai
   campi stored di tantivy stesso via query per `path`). Più isolato
   fra i due moduli, ma duplica quasi interamente l'algoritmo già
   scritto e testato per l'indice SQLite. Proposta: riuso (come sopra).
   Confermi, o preferisci l'isolamento anche a costo di duplicazione?
2. **Scorciatoia da tastiera**: Cmd/Ctrl+K (proposta, convenzione
   diffusa per "quick open/search") vs Cmd/Ctrl+F (più "cerca in
   pagina", potrebbe fuorviare) vs nessuna scorciatoia, solo bottone?
3. **Snippet con HTML (`dangerouslySetInnerHTML`)**: proposta sopra,
   sicura perché tantivy fugge tutto tranne i propri tag `<b>`
   iniettati. Alternativa più semplice (meno codice, niente
   `dangerouslySetInnerHTML` in tutto il progetto finora): snippet come
   testo semplice via `snippet.fragment()`, nessuna evidenziazione dei
   termini. Confermi l'HTML con evidenziazione, o preferisci testo
   semplice?
4. **Granularità pagina/giorno** (non blocco): proposta sopra come
   scelta senza alternative valide dato lo stato attuale della UI.
   Da confermare comunque esplicitamente, è la decisione con più
   implicazioni sullo schema.

## Test da scrivere (core)

- `SearchIndex::open` su una cartella vuota non fallisce, crea lo
  schema.
- `refresh_page` indicizza una pagina; `search` con un termine presente
  nel suo contenuto la trova.
- `search` con un termine assente da tutto il vault → lista vuota, non
  errore.
- `search` con query vuota → lista vuota, nessuna chiamata al
  `QueryParser` (evitare l'errore di parsing su stringa vuota).
- `search` con query malformata (es. virgolette sbilanciate) → lista
  vuota, non errore propagato.
- `remove_page` rimuove un documento precedentemente indicizzato: una
  ricerca che prima lo trovava, dopo non lo trova più.
- `refresh_page` chiamata due volte sulla stessa pagina (contenuto
  cambiato) non produce risultati duplicati per lo stesso path.
- Uno `Index::sync` con file nuovi/cambiati/rimossi produce un
  `SyncOutcome` coerente con `crates/ramus-core/src/index.rs` esistente
  (estensione dei test già scritti per `sync`, non nuovi da zero).
- Uno snippet generato contiene `<b>` intorno al termine cercato
  quando il termine è presente nel contenuto.

## Verifica

`cargo test` copre la parte automatizzabile del core.
`npm run typecheck` per il frontend. Non testabile in questo sandbox:
interazione da tastiera (apertura pannello, digitazione, selezione di
un risultato, navigazione verso una pagina o un giorno di journal) —
serve un giro manuale in `npm run tauri dev`.
