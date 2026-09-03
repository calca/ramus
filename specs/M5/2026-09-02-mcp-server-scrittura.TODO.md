# Server MCP — strumenti di scrittura

Stato: proposta, in attesa di conferma. Presuppone
`specs/M5/2026-09-02-mcp-server-lettura.DONE.md` già implementata e
collaudata — stesso crate `ramus-mcp`, si aggiungono strumenti, non
se ne crea un secondo.

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

1. Default di `--read-only`: scrittura **abilitata** di default
   (proposto, coerente con lo scopo del pezzo) — o preferisci il
   contrario, scrittura disabilitata finché non si passa
   esplicitamente `--write` o simile? Cambia solo quale sia il flag
   "opt-in" vs "opt-out", non le funzionalità.
2. `open_today`/`open_page` creano file se mancanti — un agente
   potrebbe crearne per errore (es. un titolo con un typo diventa una
   pagina orfana). Accettabile (stesso rischio già presente quando
   l'utente stesso clicca "Crea «query»" nell'autocomplete link, M2),
   o vuoi un qualche limite in più qui?

## Test da scrivere (core)

Nessuno nuovo in `ramus-core` (zero modifiche, `write_page`/
`open_today`/`open_page` sono già testati). In `ramus-mcp`:

- `write_page` tramite lo strumento MCP produce lo stesso file sul
  disco di una chiamata diretta a `Vault::write_page`.
- Dopo `write_page`, una `search` successiva nella stessa sessione
  trova il nuovo contenuto (verifica che `refresh_page` sull'indice
  locale di `ramus-mcp` funzioni).
- `--read-only` non registra gli strumenti di scrittura nell'elenco
  esposto (verificabile chiamando l'introspezione degli strumenti del
  server, non tentando la chiamata e aspettandosi un rifiuto).

## Verifica

`cargo test -p ramus-mcp` per quanto sopra. Il collaudo con un client
MCP reale (un agente che scrive nel vault, la GUI aperta in parallelo
che riceve l'aggiornamento) non è verificabile in questo sandbox:
previsto per domani insieme al collaudo del primo pezzo.
