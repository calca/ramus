# Impostazioni MCP: info di collegamento + interruttore on/off

Stato: implementata. Terzo pezzo di M5, dopo
`specs/M5/2026-09-02-mcp-server-lettura.DONE.md` e
`specs/M5/2026-09-02-mcp-server-scrittura.DONE.md` (entrambe già
implementate). Tutte e quattro le "Domande aperte" confermate come
proposto: `mcp_enabled` attivo di default, sezione chiamata "MCP",
solo selezione manuale del testo (nessun bottone copia, nessuna
dipendenza nuova), uscita immediata con errore su stderr quando
disabilitato.

## Motivazione

`ramus-mcp` esiste e funziona, ma oggi collegarlo richiede il
terminale: compilare, lanciare `ramus-mcp --print-config`, copiare lo
snippet a mano nel file di configurazione del client. Nessun modo di
vedere se è raggiungibile, né di spegnerlo, da dentro Ramus. Questo
pezzo aggiunge una sezione "MCP" in Impostazioni con lo snippet già
pronto (se il binario si trova) e un interruttore che disattiva
davvero il server, non solo la sezione UI.

## Interruttore: un vero kill switch, non solo estetico

Nuovo campo `Config.mcp_enabled: bool` (default: **da confermare**,
vedi "Domande aperte"), stesso `config.json` già condiviso fra GUI e
`ramus-mcp` (`Config::load_or_init()`, M5 lettura). `ramus-mcp` lo
legge all'avvio, **prima** di aprire vault/indici: se `false`, stampa
un messaggio chiaro su stderr ed esce subito (`std::process::exit(1)`),
senza mai entrare nel loop di serve — un client MCP che prova ad
avviarlo lo vede uscire immediatamente, non "connesso ma senza
strumenti".

```rust
// in ramus-mcp/src/main.rs, dopo Config::load_or_init()?, prima di
// aprire Index/SearchIndex — nessun bisogno del vault se si esce subito
if !config.mcp_enabled {
    eprintln!(
        "ramus-mcp è disabilitato (Impostazioni → MCP in Ramus). Riabilitalo per usare questo server."
    );
    std::process::exit(1);
}
```

Effetto pratico: **non istantaneo su una sessione già connessa**
(`ramus-mcp` è un processo a vita breve avviato dal client per
sessione, non un servizio residente — M5 lettura, "Avvio: indici
sempre freschi") — si applica al prossimo avvio del server da parte
del client (prossima riconnessione, o riavvio del client). Stesso
principio già accettato per `git_sync_interval_minutes` ("si applica
al prossimo tick, non istantaneo").

`--print-config` **resta utilizzabile anche a MCP disabilitato**: non
tocca il vault, serve solo a leggere il proprio percorso — nessun
motivo di bloccarlo.

## Trovare il binario dalla GUI

`ramus-mcp` e il binario dell'app Tauri sono due membri dello stesso
Cargo workspace: `cargo build`/`cargo tauri dev` li compila entrambi
nella **stessa cartella** (`target/debug/` o `target/release/` alla
radice del workspace) — file fratelli. Nuova funzione in
`src-tauri/src/commands.rs` (solo lì, non in `ramus-core`: riguarda un
dettaglio del guscio Tauri, non il dominio):

```rust
fn find_mcp_binary() -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    let dir = current.parent()?;
    let name = if cfg!(windows) { "ramus-mcp.exe" } else { "ramus-mcp" };
    let candidate = dir.join(name);
    candidate.exists().then_some(candidate)
}
```

Funziona in sviluppo (`cargo tauri dev`, `cargo build`) perché
entrambi i binari condividono la cartella `target/`. **Smette di
funzionare quando Ramus verrà pacchettizzato per la distribuzione**
(un `.app`/`.exe` impacchettato non porta con sé `ramus-mcp` a meno di
un passo di build dedicato) — accettabile perché la pacchettizzazione
è esplicitamente fuori scope sia qui sia nella spec di M5 lettura
("Pacchettizzazione/distribuzione: fuori scope per ora"). Se il
binario non si trova, la sezione Impostazioni lo dice chiaramente
(vedi sotto) invece di mostrare un percorso inventato.

## Command Tauri

```rust
#[derive(Serialize)]
pub struct McpInfo {
    pub enabled: bool,
    pub binary_found: bool,
    /// Snippet JSON pronto da incollare, `None` se `binary_found` è `false`.
    pub config_snippet: Option<String>,
}

#[tauri::command]
pub fn get_mcp_info(state: State<AppState>) -> Result<McpInfo, CoreError> { .. }

#[tauri::command]
pub fn set_mcp_enabled(enabled: bool, state: State<AppState>) -> Result<Config, CoreError> { .. }
```

`config_snippet` costruito direttamente nel command (stesso oggetto
JSON che `ramus-mcp --print-config` stampa, ~10 righe di
`serde_json::json!`) — duplicato apposta invece di richiamare il
binario come sottoprocesso e fare parsing del suo stdout: la logica è
banale, un sottoprocesso in più (con relativa gestione di errori,
timeout, encoding) non vale la complessità per dieci righe di JSON.

## Impostazioni: nuova sezione "MCP"

In `SettingsPanel`, dopo la sezione "Task":

- Un interruttore ("Abilita server MCP") — checkbox come quello del
  rollover automatico dei task, stesso pattern.
- **Se abilitato e binario trovato**: blocco di testo monospazio
  selezionabile con lo snippet JSON, più una riga di aiuto statica
  ("Incollalo in `.mcp.json` (Claude Code) o
  `claude_desktop_config.json` (Claude Desktop). Riavvia il client
  dopo una modifica.") — stesso stile della sezione Sync (testo
  d'aiuto semplice, nessun link esterno generato).
- **Se abilitato ma binario non trovato**: messaggio che spiega cosa
  fare ("Binario `ramus-mcp` non trovato — esegui `cargo build -p
  ramus-mcp` e riapri questa sezione.").
- **Se disabilitato**: solo una riga che spiega l'effetto ("Il server
  MCP si rifiuta di avviarsi finché non lo riattivi qui."), nessuno
  snippet mostrato (spegnerlo e mostrare comunque come collegarlo
  sarebbe contraddittorio).

Nessun polling: `get_mcp_info` chiamato una volta all'apertura del
pannello (il binario non compare/scompare mentre le Impostazioni sono
aperte, a differenza dello stato di sync Git che cambia da solo in
background).

## Fuori scope per questa spec

- Avviare/fermare `ramus-mcp` dalla GUI (un processo gestito da
  Ramus): `ramus-mcp` resta avviato **dal client MCP**, non da Ramus
  — coerente con "gira indipendentemente dall'app Tauri" già scritto
  in M5 lettura. L'interruttore blocca l'avvio lato server, non
  gestisce un processo.
- Generare/scrivere `.mcp.json` direttamente nel progetto dell'utente
  dalla GUI: solo mostrare lo snippet da incollare a mano, stessa
  scelta già fatta per l'URL del remote Git (M3) — Ramus non scrive
  file fuori dal vault per conto di strumenti esterni.
- Toggle per `--read-only` dentro Impostazioni: resta un argomento
  CLI passato dal client (già costruito in M5 scrittura), non
  duplicato come opzione GUI qui — se servisse in futuro, spec a
  parte.
- Verificare attivamente se un client è connesso in questo momento
  (nessuna GUI di Ramus vede lo stato di connessione di `ramus-mcp`,
  che è un processo separato e a vita breve).

## Domande aperte

Nessuna: tutte e quattro confermate come proposto (vedi cima del
documento).

## Test da scrivere

**Core** (`crates/ramus-core/src/config.rs`): stesso trattamento degli
altri campi — `config_without_mcp_enabled_field_defaults_to_true`,
verifica il default quando il campo manca dal `config.json` (config
scritti da versioni precedenti a questa spec).

**`ramus-mcp`**: `mcp_disabled_message_is_none_when_enabled_and_some_when_disabled`
— la logica "abilitato → nessun messaggio, disabilitato → messaggio da
stampare" è isolata in una funzione pura (`mcp_disabled_message`,
separata da `main`) proprio per poter essere testata senza chiamare
`std::process::exit` per davvero (ucciderebbe il processo di test).
`main()` chiama questa funzione e fa l'uscita reale, non testato
direttamente (stesso principio già seguito altrove nel progetto: la
logica testabile sta in una funzione pura, il guscio attorno resta
sottile).

**Frontend**: nessun test nuovo, coerente con l'assenza di un runner
JS per componenti nel progetto (stessa scelta di tutte le altre
sezioni di `SettingsPanel`).

## Verifica

`cargo test` (115 core + 15 mcp, +1 e +1 da questa spec), `cargo
clippy --all-targets -D warnings`, `cargo fmt --check`, `npm run
typecheck` — tutti puliti. Il rilevamento del binario fratello è
verificato per davvero in questo sandbox (non solo per ispezione del
codice): `ls target/debug/` conferma `ramus` e `ramus-mcp` nella
stessa cartella, esattamente l'assunzione su cui si basa
`find_mcp_binary()`. Non verificabile qui: il giro completo in `npm
run tauri dev` con Impostazioni aperte (aspetto della sezione,
comportamento dei tre stati enabled/binario-mancante/disabled) —
osservata mentre si compilava `ramus-mcp` in questa sessione). Non
verificabile: il giro completo in `npm run tauri dev` con Impostazioni
aperte (aspetto della sezione, comportamento dei tre stati
enabled/binario-mancante/disabled) — richiede un giro manuale.
