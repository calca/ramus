# Indice SQLite (pagine, blocchi, link)

Stato: implementata (`crates/ramus-core/src/index.rs`). `position`
piatto, sincronizzazione via mtime — pensata per vault grandi fin da
questa versione. Nessuna UI: `find_backlinks`/`list_tags` sono
raggiungibili da command Tauri ma non consumati da nessun componente,
come previsto (pannello backlink e autocomplete tag restano spec a
parte).

## Motivazione

Secondo pezzo di M2 (SPEC.md): "Indice SQLite rigenerabile con pagine,
link e blocchi". Già pre-approvato nello Stack di SPEC.md ("SQLite via
`rusqlite` — indice derivato dalla milestone 2"). Sblocca il terzo
pezzo (pannello backlink, spec separata) e chiude un debito lasciato
aperto dalla spec sui link: l'autocomplete dei tag, rimandato
esplicitamente "all'indice SQLite".

**Principio non negoziabile di SPEC.md #1**: "I file markdown sono la
source of truth... indice e cache sono derivati e rigenerabili: se il
codice non funziona dopo aver cancellato l'indice, è rotto." Questa
spec tratta quel vincolo come test d'accettazione letterale, non solo
come principio ispiratore.

## Dove vive il file

`<vault>/.ramus/index.sqlite3` — **dentro** il vault, non nella cartella
di configurazione dell'app. Motivazione: un indice per vault, non un
indice unico multi-vault da tenere sincronizzato con quale vault è
attivo (Ramus non ha multi-vault, ma può *cambiare* vault attivo dalle
Impostazioni — l'indice deve seguire quale cartella è "il vault" senza
bisogno di hashing di percorsi o bookkeeping in `Config`). Stesso
pattern di Obsidian, che tiene la propria cartella `.obsidian/` dentro
il vault — coerente con SPEC.md, principio 4 ("compatibile con
Obsidian").

`list_pages`/`list_journals`/`stats` già guardano solo dentro
`pages/`/`journals/`: una cartella sorella `.ramus/` alla radice del
vault è automaticamente invisibile a quelle funzioni, nessun filtro da
aggiungere. Chi sincronizza il vault con git (fuori scope, M3) vorrà
un `.gitignore` con `.ramus/` — nota per allora, non una cosa da
costruire ora.

## Dipendenza nuova: `rusqlite`

Già pre-approvata in SPEC.md — non serve giustificarla di nuovo, solo
fissare la versione: `rusqlite = { version = "0.40", features =
["bundled"] }`. `bundled` compila SQLite da sorgente invece di
richiedere la libreria di sistema: essenziale per un'app cross-platform
(macOS/Windows/Linux) da distribuire senza dipendere da cosa ha
installato chi la usa.

## Schema

```sql
CREATE TABLE pages (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,   -- "pages/progetto-x.md" o "journals/2026-09-02.md"
    kind TEXT NOT NULL,          -- 'page' | 'journal'
    title TEXT,                  -- dal front-matter; NULL per i journal
    mtime INTEGER NOT NULL       -- mtime del file al momento dell'indicizzazione (secondi, epoch)
);

CREATE TABLE blocks (
    id INTEGER PRIMARY KEY,
    page_id INTEGER NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,   -- ordine di attraversamento dell'albero, per stabilità
    content TEXT NOT NULL
);

CREATE TABLE links (
    id INTEGER PRIMARY KEY,
    source_block_id INTEGER NOT NULL REFERENCES blocks(id) ON DELETE CASCADE,
    target_title TEXT NOT NULL   -- il testo esatto fra [[ ]], non ancora risolto a una pagina
);

CREATE TABLE tags (
    id INTEGER PRIMARY KEY,
    block_id INTEGER NOT NULL REFERENCES blocks(id) ON DELETE CASCADE,
    tag TEXT NOT NULL            -- senza il #
);
```

**Nota su "niente ID sui blocchi" (SPEC.md, formato su disco)**: quella
regola riguarda il formato *su file markdown* — non si scrivono ID
dentro `journals/*.md`/`pages/*.md`. Gli `id` di questo schema sono
`rowid` interni a un database derivato e rigenerabile, mai scritti sul
disco del vault: non è la stessa cosa, non la contraddice.

`links.target_title` non è una foreign key verso `pages`: il testo fra
`[[ ]]` può riferirsi a una pagina non ancora creata (link "promesso",
vedi spec sui link). La risoluzione a una pagina avviene a query time
via `slugify(target_title)`, non a scrittura.

## Estrazione link/tag: scritta a mano, niente `regex`

Stesso principio di `parser.rs`/`frontmatter.rs`: niente libreria di
parsing/regex nuova per un pattern semplice e delimitato. Scansione
con `str::find`/slicing (sempre su confini UTF-8 validi, mai indicizzazione
byte a mano) invece di espressioni regolari:

```rust
/// Testi dentro [[ ]] in una stringa. Stesso set di caratteri per tag
/// del frontend (linkTagHighlight.ts): deve restare in sincrono a mano,
/// due implementazioni indipendenti in due linguaggi.
pub fn extract_links(content: &str) -> Vec<String> { ... }

/// Tag (senza #): alfanumerico ASCII, underscore, trattino — stesso
/// set di `/#[\w-]+/` lato frontend (che senza flag `u` è già solo
/// ASCII, non full-unicode).
pub fn extract_tags(content: &str) -> Vec<String> { ... }
```

## Costruzione e aggiornamento

### Sincronizzazione all'apertura del vault (non un rebuild cieco)

All'apertura di un vault (avvio app, o cambio vault dalle
Impostazioni): **`Index::sync`**, non un `DROP`+`CREATE`+rilettura di
tutto. Pensata fin da qui per vault grandi (centinaia o migliaia di
file), non un rebuild completo aggiunto "per ora" e da rivedere dopo:

1. Si elencano tutti i file attuali in `journals/`/`pages/` (stessa
   logica di `list_journals`/`list_pages`) col loro mtime da
   filesystem (`fs::metadata(path)?.modified()`) — solo `stat()`, mai
   una lettura del contenuto in questa fase.
2. Per ogni file:
   - **assente nell'indice** → si legge, si estrae, si inserisce (righe
     nuove in `pages`/`blocks`/`links`/`tags`).
   - **presente ma mtime diverso da quello registrato** → si tratta
     come "cambiato": si cancellano le sue righe esistenti e si
     re-inserisce dal contenuto attuale (stessa logica di
     `refresh_page`, vedi sotto — `sync` per ogni file toccato
     internamente chiama la stessa primitiva).
   - **presente e mtime combacia** → si salta, nessuna lettura del
     contenuto, nessuna query di scrittura.
3. Ogni riga di `pages` il cui `path` **non esiste più** su disco
   (file cancellato mentre l'app non guardava) viene rimossa (cascade
   su `blocks`/`links`/`tags`).

Il costo di un avvio "a freddo" diventa *N* `stat()` (economici) più
la lettura/estrazione solo dei file davvero cambiati, non dell'intero
vault — la differenza pratica per un vault grande fra "ricostruire
tutto" e "sincronizzare" è proprio in questo passo.

Caso particolare, non un ramo di codice a parte: se l'indice è appena
stato creato (file assente, o appena droppato per mismatch di
versione — vedi sotto), **ogni** file risulta "assente nell'indice" e
`sync` lo indicizza tutto da zero — è lo stesso identico algoritmo,
solo applicato quando la tabella `pages` è vuota. Non serve una
funzione `rebuild` separata da `sync`.

### Versione di schema

`PRAGMA user_version` invece di un framework di migrazioni: se il
numero non combacia una costante `SCHEMA_VERSION` nel codice, si
`DROP`+`CREATE` le tabelle (tornano vuote) e si procede con lo stesso
`sync` di sopra, che a quel punto reindicizza tutto perché non trova
corrispondenze. Coerente con "derivato e rigenerabile": non serve
preservare i dati di un vecchio schema, serve solo rigenerarli nel
nuovo.

### Aggiornamenti incrementali durante la sessione

Ogni scrittura di una pagina/journal (`write_page`, `open_today`,
`open_page`) aggiorna **solo quella pagina** nell'indice: cancella le
sue righe in `blocks`/`links`/`tags` (cascade da `pages`) e le
re-inserisce dal contenuto appena scritto. Stesso trattamento per i
cambiamenti esterni rilevati dal file watcher (già letti e
ricaricati nell'interfaccia — si aggancia lì lo stesso refresh).

Nessuna modifica alle firme dei command esistenti: l'aggiornamento
dell'indice è un passo in più nel command layer dopo la chiamata al
core (`vault.write_page(...)?; index.refresh_page(...)?;`), non dentro
`Vault` stesso — `Vault` e l'indice restano moduli separati (vedi
sotto), coerente con la tabella di SPEC.md che tiene `vault.rs` per
"gestione cartella, path, creazione file", non indicizzazione.

## Modulo nuovo: `ramus-core/src/index.rs`

Non dentro `Vault`: un `Index` a parte, che apre/possiede la propria
connessione SQLite. `AppState` guadagna `index: Mutex<Index>` accanto a
`config`/`watcher`, ricreato quando il vault cambia (stesso momento in
cui si ricrea il watcher in `set_vault_path`).

```rust
pub struct Index {
    conn: rusqlite::Connection,
}

impl Index {
    pub fn open(vault_root: &Path) -> Result<Self, CoreError> { ... }
    /// Confronta i file del vault con quanto registrato (mtime) e
    /// aggiorna solo ciò che è cambiato. Su indice vuoto/appena
    /// creato, indicizza tutto — stesso algoritmo, non un caso a parte.
    pub fn sync(&self, vault: &Vault) -> Result<(), CoreError> { ... }
    pub fn refresh_page(&self, vault: &Vault, relative_path: &str) -> Result<(), CoreError> { ... }
    pub fn find_backlinks(&self, target_title: &str) -> Result<Vec<Backlink>, CoreError> { ... }
    pub fn list_tags(&self) -> Result<Vec<String>, CoreError> { ... }
}

pub struct Backlink {
    pub source_path: String,
    pub source_title: Option<String>,
    pub block_content: String,
}
```

`find_backlinks`/`list_tags` sono la prova che l'indice funziona
davvero, non solo che si scrive — ma **restano senza UI** in questa
spec (vedi "Fuori scope"): la esporrà la spec del pannello backlink.
`list_tags` è incluso perché chiude il debito lasciato aperto
esplicitamente dalla spec sui link — non serve una spec a parte per
questo pezzo, ma **l'autocomplete di `#tag` in editor resta comunque
fuori scope qui** (richiede una seconda estensione `@tiptap/suggestion`
simile a `linkAutocomplete.ts`, lavoro di frontend a parte, non
implicito nell'avere la query disponibile).

## Command Tauri

```
find_backlinks(target_title: String) -> Result<Vec<Backlink>, CoreError>
list_tags() -> Result<Vec<String>, CoreError>
```

Wrapper sottili, stesso schema degli altri. Nessun command per
rebuild/refresh: quelli sono interni, agganciati agli altri command
esistenti (setup dell'app, `set_vault_path`, `write_page`,
`open_today`, `open_page`, il gestore del file watcher).

## Fuori scope per questa spec

- Pannello backlink (UI): spec a parte, consuma `find_backlinks`.
- Autocomplete di `#tag` in editor: query disponibile (`list_tags`),
  estensione Tiptap per usarla no — lavoro di frontend a parte.
- Ricerca full-text (`tantivy`): indice diverso, per un lavoro diverso
  (ricerca testuale libera, non grafo di link) — ultimo pezzo di M2,
  non tocca questa spec.
- Sincronizzazione dell'indice fra processi/finestre multiple: Ramus
  non supporta più finestre, non applicabile.
- Watch dei file a grana fine per evitare del tutto lo `stat()` di ogni
  file a ogni avvio (es. un manifest persistito fra sessioni): lo
  `stat()` su tutti i file resta comunque necessario per rilevare
  cancellazioni avvenute a app chiusa, quindi non c'è un modo di
  evitarlo del tutto — non vale la complessità aggiuntiva ora.

## Domande aperte

Nessuna: `position` piatto e sincronizzazione via mtime sono decisioni
prese (vedi sopra).

## Test da scrivere (core)

- `extract_links`/`extract_tags`: casi base, stringa senza match,
  parentesi/cancelletto senza chiusura o isolati, contenuto con
  caratteri unicode multi-byte accanto ai delimitatori (niente panico
  sui confini UTF-8), tag consecutivi.
- `Index::sync` su indice vuoto e un vault con pagine/journal/link/tag
  noti produce esattamente le righe attese in tutte e quattro le
  tabelle (il caso "indicizza tutto da zero").
- `Index::sync` su un vault vuoto non fallisce, produce tabelle vuote.
- Cancellare il file dell'indice e riaprire il vault lo ricostruisce
  identico (principio 1 di SPEC.md, testato alla lettera).
- `Index::sync` chiamato due volte di seguito senza toccare i file non
  duplica righe (nessuna scrittura sui file invariati — verificabile
  anche solo controllando che i conteggi restino identici).
- `Index::sync` dopo aver modificato un file esternamente (mtime
  cambiato) aggiorna solo quella pagina, lasciando le altre invariate.
- `Index::sync` dopo aver cancellato un file esternamente rimuove le
  sue righe (cascade su blocchi/link/tag), lascia intatto il resto.
- `refresh_page` su una pagina già indicizzata sostituisce le sue righe
  invece di duplicarle (nessun accumulo a ogni salvataggio).
- `find_backlinks` trova un link scritto in un journal verso una
  pagina, e viceversa (i link possono partire da entrambi i tipi).
- `find_backlinks` non trova nulla per un titolo mai linkato — lista
  vuota, non errore.
- `list_tags` deduplica (`#idea` scritto in due punti compare una sola
  volta) ed è ordinata.

## Verifica

`cargo test` copre la costruzione/query dell'indice. Non testabile in
questo sandbox: comportamento a runtime dentro l'app vera (sync
all'avvio, refresh dopo un salvataggio reale) — verificabile con lo
stesso script usa-e-getta (`cargo run -p ramus-core --example`) già
usato altre volte in questa sessione, oltre a un giro in
`npm run tauri dev`.
