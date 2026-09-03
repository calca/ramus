# Server MCP — strumenti di scrittura

Stato: implementata. Presuppone
`specs/M5/2026-09-02-mcp-server-lettura.DONE.md` già implementata e
collaudata — stesso crate `ramus-mcp`, si aggiungono strumenti, non
se ne crea un secondo. Entrambe le "Domande aperte" confermate come
proposto: scrittura abilitata di default (`--read-only` per
escluderla), stesso rischio già accettato per la creazione di pagine
al volo (nessun limite aggiuntivo).

## Motivazione

Secondo pezzo di M5: senza scrittura l'agente può solo consultare il
vault, non usarlo davvero ("l'agente interacisce con Ramus" era la
richiesta originale). Separato dal primo pezzo perché consequenziale
in un modo che la sola lettura non è — un errore in uno strumento di
lettura dà una risposta sbagliata, un errore in uno di scrittura tocca
i file dell'utente.

## Concorrenza con la GUI: già gestita, nessun lavoro nuovo

Due meccanismi già esistenti coprono esattamente questo caso, senza
bisogno di scrivere nulla di nuovo:

1. **Se la GUI è aperta mentre `ramus-mcp` scrive**: il file watcher
   (`notify`, M1) osserva il vault indipendentemente da quale processo
   lo modifica. Il callback in `spawn_watcher`
   (`src-tauri/src/commands.rs`) già aggiorna `Index`/`SearchIndex`
   della GUI per **qualunque** file cambiato esternamente — costruito
   per i pull di M3 e per edit da altri strumenti (Obsidian), copre
   `ramus-mcp` senza modifiche. Il frontend riceve lo stesso evento
   `vault://file-changed` di sempre: ricarica se la sezione non ha
   modifiche pendenti, avvisa senza sovrascrivere se le ha.
2. **Se `ramus-mcp` scrive e poi legge/cerca nella stessa sessione**:
   `SearchIndex` in modalità GUI usa già `ReloadPolicy::Manual` con
   `reload()` esplicito a ogni ricerca (M2) — legge sempre fresco da
   disco, a prescindere da quale processo ha scritto per ultimo.
   SQLite gestisce nativamente le scritture da processi diversi sullo
   stesso file. Nessuna sincronizzazione fra processi da costruire.

## Strumenti aggiunti

| Strumento MCP | Funzione `ramus-core` | Effetto |
| --- | --- | --- |
| `write_page` | `Vault::write_page(path, blocks)` | sovrascrive i blocchi di una pagina esistente |
| `open_today` | `Vault::open_today()` | crea il journal di oggi se manca, lo apre |
| `open_page` | `Vault::open_page(name)` | crea la pagina se manca (stesso slug di sempre), la apre |

Dopo ogni scrittura, stesso schema già in
`src-tauri/src/commands.rs`: `vault.write_page(...)?` seguito da
`index.refresh_page(...)?` e `search_index.refresh_page(...)?` sugli
handle aperti da `ramus-mcp` stesso — l'indice del **suo** processo
resta coerente per chiamate successive nella stessa sessione, non solo
quello della GUI (che si aggiorna comunque via watcher, vedi sopra).

### Struct dei parametri: `Block` locale a `ramus-mcp`

`ramus_core::Block` non deriva `JsonSchema` (non deve — `ramus-core`
non dipende da `schemars`, stessa regola di CLAUDE.md già rispettata
per Tauri). `ramus-mcp` definisce un tipo locale, stesso principio già
seguito da `src/lib/types.ts` nel frontend ("i tipi dei command
rispecchiano le struct Rust, tenuti allineati a mano" — qui un terzo
client, stessa disciplina):

```rust
#[derive(Deserialize, JsonSchema)]
struct BlockInput {
    content: String,
    #[serde(default)]
    children: Vec<BlockInput>,
}

impl From<BlockInput> for ramus_core::Block {
    fn from(input: BlockInput) -> Self {
        ramus_core::Block {
            content: input.content,
            children: input.children.into_iter().map(Into::into).collect(),
        }
    }
}
```

## `--read-only`: un modo di escludere questi strumenti del tutto

Flag da riga di comando su `ramus-mcp` (default: scrittura abilitata
— coerente con lo scopo dichiarato di questo pezzo): se passato,
il server registra solo gli strumenti del primo pezzo, quelli di
scrittura non compaiono nemmeno nell'elenco esposto al client MCP
(non "rifiutati a runtime", proprio assenti). Per chi vuole collegare
un agente solo per interrogare il proprio journal senza mai rischiare
una scrittura, senza doversi fidare del client MCP per non chiamarli.

```json
{
  "command": "/path/to/ramus-mcp",
  "args": ["--read-only"]
}
```

**Meccanica implementativa**: due `#[tool_router]` distinti sullo
stesso `impl Server` (uno per la lettura, `read_tool_router`, uno per
la scrittura, `write_tool_router` — invece di uno solo come nel primo
pezzo), uniti in `Server::new` con `ToolRouter::merge`/`+=` solo se
`!read_only`. Gli strumenti esclusi non esistono proprio nel router
finale, non solo "disabilitati" — coerente con "assenti dall'elenco",
non "rifiutati a runtime".

`ramus-mcp --print-config` (dal primo pezzo) ricorda anche questa
opzione nel testo che stampa, non solo nell'esempio statico qui sopra.

## Fuori scope per questa spec

- Conferma/anteprima prima di ogni scrittura (un "vuoi davvero
  scrivere questo?" dentro Ramus): l'autorizzazione vive nel client
  MCP (Claude Desktop/Code hanno già un proprio flusso di conferma
  per le tool call) — Ramus non duplica quel livello.
- Cronologia/undo delle scritture fatte da un agente: nessuna
  distinzione fra una scrittura arrivata da MCP e una dalla GUI, sono
  lo stesso `write_page`. Se serve un log/undo, è una feature
  generale (non specifica a MCP), fuori scope qui.
- Eliminare pagine o journal: nessuno strumento `delete_*` — la
  GUI stessa non offre questa azione oggi, non la si introduce prima
  qui che lì.
- Strumenti per rinominare una pagina (cambiare `title` nel
  front-matter dopo la creazione): nessuna UI lo fa oggi nella GUI
  (fuori scope già in `specs/M2/2026-09-02-link-tag-parsing.DONE.md`),
  stessa esclusione qui.

## Domande aperte

Nessuna: entrambe confermate come proposto (vedi cima del documento).

## Test da scrivere (core)

Nessuno nuovo in `ramus-core` (zero modifiche, confermato —
`write_page`/`open_today`/`open_page` sono già testati lì). In
`ramus-mcp`, 6 test aggiuntivi (totale 14 con quelli del primo
pezzo):

- `write_page` tramite lo strumento MCP produce lo stesso file sul
  disco di una chiamata diretta a `Vault::write_page`.
- Dopo `write_page`, una `search` successiva nella stessa sessione
  trova il nuovo contenuto (verifica che `refresh_page` sull'indice
  locale di `ramus-mcp` funzioni).
- `open_today` crea il file del journal di oggi se manca.
- `open_page` crea una pagina col titolo indicato.
- `--read-only` non registra gli strumenti di scrittura nell'elenco
  esposto (`ToolRouter::has_route`, introspezione diretta — non un
  tentativo di chiamata che ci si aspetta venga rifiutato).
- Scrittura abilitata di default: senza `--read-only`, tutti e tre
  gli strumenti di scrittura sono nel router.

## Verifica

**Aggiornamento rispetto al testo originale**: verificato prima del
previsto, non rimandato a domani. `cargo test -p ramus-mcp` (14
test), `cargo test` sull'intero workspace (114 core + 14 mcp),
`cargo clippy --all-targets -D warnings` e `cargo fmt --check` puliti.
Collaudo end-to-end reale con richieste JSON-RPC grezze su stdio,
contro un vault isolato e usa-e-getta (mai il vault reale
dell'utente — `HOME` sovrascritto solo per il sottoprocesso, così
`Config::load_or_init()` risolve un `config.json` completamente
separato): `open_today` (crea il file), `write_page` (scrive e
rilegge correttamente), `search` subito dopo (trova il contenuto
appena scritto — conferma che `refresh_page` sull'indice del
processo `ramus-mcp` funziona anche attraverso il transport reale, non
solo nella chiamata diretta ai metodi), `open_page` (crea con
front-matter corretto). Non verificato: un client MCP completo (Claude
Desktop/Code) e lo scenario "GUI aperta in parallelo riceve
l'aggiornamento via watcher" — quest'ultimo si appoggia a un
meccanismo (`spawn_watcher`) già testato per altri casi (M3), non
ri-testato qui.
