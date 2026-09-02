# Server MCP — strumenti di sola lettura

Stato: proposta, in attesa di conferma. Primo pezzo di M5. Il secondo
pezzo (`specs/M5/2026-09-02-mcp-server-scrittura.TODO.md`, strumenti
che modificano il vault) presuppone questo già collaudato.

## Motivazione

M5 non è più "Ramus chiama un'AI" ma il contrario: **Ramus si espone
a un agente/tool AI** (Claude Code, Claude Desktop, o qualunque client
MCP) invece di integrare un provider al suo interno. Il vincolo già
scritto in SPEC.md ("nessun invio automatico di contenuti a servizi
esterni... ogni chiamata esplicita") è soddisfatto quasi per
definizione con questo approccio: Ramus non contatta mai un'AI, è
l'agente — già configurato e autorizzato dall'utente per conto suo —
a leggere/scrivere nel vault tramite Ramus. Nessuna chiave API, nessun
provider da gestire dentro l'app.

Questo pezzo copre solo la lettura: cercare, elencare, leggere pagine
e journal, backlink, tag. Nessuna modifica al vault — il rischio è
minimo, si può collaudare subito.

## Perché `ramus-core` è già pronto per questo

CLAUDE.md, regola 1: "`ramus-core` non dipende da Tauri... deve
compilare e testarsi da solo." SPEC.md, principio 2: "esposto in
futuro via FFI a un client mobile." Un server MCP è esattamente
questo: un **nuovo consumatore** di `ramus-core`, indipendente da
Tauri, senza toccare una riga del crate esistente. Nessuna modifica a
`ramus-core` prevista in questa spec.

## Nuovo crate: `ramus-mcp`

Binario a sé, nuovo membro del workspace (`crates/ramus-mcp`),
accanto a `ramus-core` e `src-tauri` — non dentro nessuno dei due.
Gira **indipendentemente dall'app Tauri**: funziona anche a GUI
chiusa, letto/scritto direttamente sul filesystem del vault via
`ramus-core`, nessuna comunicazione con il processo dell'app grafica
(nessun socket, nessun IPC fra i due — solo lo stesso vault su disco).

### Dipendenza nuova: `rmcp`

```toml
# solo in crates/ramus-mcp/Cargo.toml, MAI in ramus-core (vedi sopra)
rmcp = { version = "3", features = ["server", "transport-io", "macros", "schemars"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-std"] }
schemars = "0.8"
```

Verificato: `rmcp` è l'SDK Rust ufficiale del progetto Model Context
Protocol (`github.com/modelcontextprotocol/rust-sdk`, la stessa
organizzazione dello standard). `transport-io` è il transport stdio
lato server — lo standard per un binario invocato da un client MCP
(Claude Desktop, Claude Code: entrambi lanciano il comando configurato
e parlano MCP sul suo stdin/stdout). `macros` dà le attribute macro
`#[tool]`/`#[tool_router]`/`#[tool_handler]` per dichiarare gli
strumenti senza scrivere a mano il routing/schema JSON.

### Trovare il vault

`ramus_core::Config::load_or_init()` — **la stessa funzione già usata
dall'app Tauri**, stesso file (`config.json`, percorso già gestito da
`Config::config_file_path()`). `ramus-mcp` punta sempre allo stesso
vault della GUI, senza configurazione separata: se l'utente ha aperto
Ramus almeno una volta, `ramus-mcp` sa già dove guardare. Nessun
override (env var, flag) per un vault diverso — coerente con "niente
multi-vault" (SPEC.md, Fuori scope): un solo vault, sempre quello
configurato.

### Avvio: indici sempre freschi, anche a GUI chiusa

A differenza dell'app Tauri (che tiene `Index`/`SearchIndex` vivi in
`AppState` e li aggiorna a ogni scrittura), `ramus-mcp` è un processo
a vita breve, avviato dal client MCP: all'avvio fa lo stesso lavoro
già presente in `src-tauri/src/lib.rs::run` — `Index::open` +
`Index::sync` (che restituisce `SyncOutcome`, M2), poi
`SearchIndex::open` + `refresh_page`/`remove_page` per i path in
`outcome.refreshed`/`outcome.removed`. Stessa sequenza, stesso codice
concettuale, copiato non condiviso (non vale la pena estrarre
un'astrazione per due chiamate identiche in due binari diversi).

Questo garantisce che `search`/`find_backlinks`/`list_tags` siano
corretti anche se il vault è stato modificato da un altro processo
(Obsidian, `git pull` di M3, o la stessa GUI chiusa da tempo) prima
che l'agente si connetta.

## Strumenti esposti

Ognuno è un wrapper sottile su una funzione già esistente in
`ramus-core` — stessa logica dei command Tauri in
`src-tauri/src/commands.rs`, stesso principio ("solo wrapper, nessuna
decisione qui") applicato a un secondo client invece che duplicato:

| Strumento MCP | Funzione `ramus-core` | Note |
| --- | --- | --- |
| `list_journals` | `Vault::list_journals(before, limit)` | `before` stringa ISO opzionale |
| `read_page` | `Vault::read_page(path)` | path relativo al vault |
| `list_pages` | `Vault::list_pages()` | slug + titolo di tutte le pagine |
| `search` | `SearchIndex::search(query)` | full-text, M2, invariato |
| `find_backlinks` | `Index::find_backlinks(target_title)` | M2, invariato |
| `list_tags` | `Index::list_tags()` | M2, invariato |

Ogni strumento ha una struct parametri locale in `ramus-mcp` (non
riusata da `ramus-core`, che non deve dipendere da `schemars`):

```rust
#[derive(Deserialize, JsonSchema)]
struct SearchParams {
    /// Testo da cercare nel vault (titoli e contenuto dei blocchi).
    query: String,
}

#[tool(name = "search", description = "Cerca full-text nel vault (pagine e journal).")]
async fn search(&self, params: Parameters<SearchParams>) -> String {
    // apre/usa il SearchIndex già costruito all'avvio, serializza
    // Vec<SearchHit> (già Serialize, M2) in JSON come risultato
}
```

`#[tool_router]`/`#[tool_handler]` (macro di `rmcp`) generano il
routing verso `ServerHandler` — stesso schema visto nei test del
crate (`Server { tool_router: ToolRouter<Self> }`,
`#[tool_router(router = tool_router)]`,
`#[tool_handler(router = self.tool_router)]`).

## Cosa resta fuori da questa spec

- **Nessuno strumento che crea o modifica file**: `open_today` e
  `open_page` hanno un effetto collaterale di scrittura (creano il
  file se manca) — non sono "di sola lettura" in senso stretto, vanno
  nel secondo pezzo insieme a `write_page`. `list_journals(before:
  None, limit: 1)` copre già "cosa ho scritto di recente" senza
  bisogno di uno strumento dedicato a "oggi".
- Watcher/notifiche in tempo reale verso l'agente (es. "il vault è
  cambiato mentre eri connesso"): ogni chiamata legge fresco da
  disco/indice al momento della chiamata, non serve un meccanismo di
  push per uno strumento di sola lettura.
- Autenticazione/autorizzazione sul server MCP: gira in locale, sul
  filesystem dell'utente, invocato dal suo stesso client MCP già
  fidato — nessun livello di autenticazione aggiuntivo, coerente con
  "niente account utente" (SPEC.md, Fuori scope).
- Pacchettizzazione/distribuzione (binario firmato, installer): fuori
  scope per ora, si costruisce con `cargo build` come gli altri
  membri del workspace.

## Domande aperte

1. Nome del crate/binario: `ramus-mcp` (proposto) — va bene?
2. Il file di configurazione per i client MCP (es. `.mcp.json` per
   Claude Code, `claude_desktop_config.json` per Claude Desktop) non
   è generato da Ramus: l'utente lo scrive a mano puntando al
   percorso del binario compilato. Va bene per ora (nessuna UI di
   "genera configurazione" dentro Ramus), o vuoi che una spec futura
   copra anche quello?

## Test da scrivere (core)

Nessuno nuovo in `ramus-core` (zero modifiche). In `ramus-mcp`: test
di integrazione che aprono un vault di prova (stesso pattern
`TempDir` già usato in `ramus-core`), avviano gli strumenti
direttamente (senza il transport MCP, chiamando le funzioni Rust
sottostanti) e verificano che restituiscano gli stessi dati dei
command Tauri equivalenti già testati.

## Verifica

`cargo test` per `ramus-mcp` (quanto sopra) e `cargo build -p
ramus-mcp`. Il collaudo reale end-to-end (un client MCP che si
connette, elenca gli strumenti, li invoca) non è verificabile in
questo sandbox: previsto per domani, un giro con un client MCP reale
(es. Claude Code puntato al binario compilato).
