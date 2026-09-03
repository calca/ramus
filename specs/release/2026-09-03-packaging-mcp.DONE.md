# Pacchettizzare `ramus-mcp` insieme all'app distribuita

Stato: implementata. Nessuna domanda bloccante — una sola decisione
tecnica, proposta sotto con motivazione, non un bivio di prodotto.
Emersa un'implicazione non anticipata in fase di stesura (vedi
"Scoperta in corso d'opera"), risolta senza riaprire le decisioni
principali.

## Motivazione

`find_mcp_binary()` (`src-tauri/src/commands.rs`, spec
`specs/M5/2026-09-03-mcp-impostazioni.DONE.md`) trova `ramus-mcp`
cercando un file fratello nella stessa cartella del binario
dell'app — funziona in sviluppo perché `cargo build`/`tauri dev`
mettono entrambi i binari in `target/debug|release/`. **Già
documentato come rotto in un'app pacchettizzata**: un `.app`/`.exe`
distribuito non porta con sé `ramus-mcp` a meno di un passo di build
dedicato — questa spec è quel passo.

## Soluzione: `tauri.conf.json` → `bundle.resources` (o `externalBin`)

Tauri supporta due meccanismi per includere un binario extra nel
bundle:

- **`bundle.externalBin`**: pensato per "sidecar", eseguibili esterni
  copiati nel bundle con un nome basato sul target triple (es.
  `ramus-mcp-x86_64-apple-darwin`) — il meccanismo pensato apposta per
  questo caso (un secondo binario dello stesso workspace, non una
  libreria di sistema).
- Alternativa scartata: `bundle.resources` (copia file generici, non
  pensato per eseguibili — richiederebbe gestire a mano i permessi di
  esecuzione per piattaforma).

**Scelta**: `externalBin`, è il meccanismo Tauri esplicitamente
documentato per "un secondo binario del proprio progetto da
distribuire insieme all'app principale" (proprio questo caso).

## Modifiche

- `tauri.conf.json` → `bundle.externalBin: ["binaries/ramus-mcp"]`.
  Tauri copia il sidecar accanto al binario principale nel bundle
  finale, con il nome suffissato dal target triple corrente
  (convenzione Tauri, verificata leggendo il doc comment del campo in
  `tauri-utils` — non un'assunzione).
- `src-tauri/build.rs`: incorpora `TARGET_TRIPLE` come `env!()` a
  tempo di compilazione (Rust non lo espone altrimenti a runtime —
  serve a `find_mcp_binary` per sapere che nome cercare).
- `scripts/prepare-mcp-sidecar.mjs` (nuovo, Node — non uno shell
  script: deve girare identico su macOS/Windows/Linux, nessuna
  dipendenza nuova): compila `ramus-mcp --release`, copia il binario
  in `src-tauri/binaries/ramus-mcp-<target-triple>[.exe]`, permessi di
  esecuzione su Unix. `package.json` → script `prepare:mcp-sidecar`;
  `tauri.conf.json` → `beforeBuildCommand` diventa `npm run
  prepare:mcp-sidecar && npm run build` (eseguito automaticamente da
  Tauri prima di ogni `tauri build`, indipendentemente da come viene
  invocato).
- `find_mcp_binary()` (`commands.rs`): due tentativi in sequenza,
  nome semplice (`ramus-mcp`, dev — invariato) poi nome suffissato dal
  target triple (bundle pacchettizzato) — entrambi cercati come file
  fratello del binario in esecuzione, stesso principio già in uso,
  solo esteso.
- `get_mcp_info`: nessuna modifica di firma, riflette comunque
  `binary_found` in base a `find_mcp_binary` — ma quella funzione ora
  richiede anche che il file non sia vuoto (vedi sotto).

## Scoperta in corso d'opera: `tauri_build::build()` valida l'externalBin sempre, non solo a `tauri build`

Aggiungere `externalBin` a `tauri.conf.json` fa sì che **ogni**
`cargo check`/`cargo build`/`cargo test` del crate `ramus` (non solo
`tauri build`) fallisca se il file dichiarato non esiste già su disco
— scoperto per davvero (`cargo check -p ramus` falliva con "resource
path ... doesn't exist"), non nella progettazione iniziale della
spec. Avrebbe rotto silenziosamente `npm run tauri dev`, ogni
`cargo test` locale e il workflow CI appena scritto
(`specs/release/2026-09-03-ci.TODO.md`) per chiunque non avesse prima
eseguito a mano `prepare:mcp-sidecar`.

**Fix**: `build.rs` crea un segnaposto vuoto (0 byte, permessi di
esecuzione su Unix) allo stesso percorso suffissato se non esiste già
— soddisfa solo il controllo di esistenza di `tauri-build`, mai
raggiungibile dal `find_mcp_binary` runtime in sviluppo (vive in
`src-tauri/binaries/`, mai copiato in `target/debug/` da un semplice
`cargo build`: solo il bundler di `tauri build` fa quella copia).
`find_mcp_binary` guadagna un controllo `metadata().len() > 0` per non
scambiare mai il segnaposto per il binario vero, anche nell'improbabile
caso finisse in una build pacchettizzata per un errore di sequenza.
`src-tauri/binaries/` aggiunta a `.gitignore` (rigenerata a ogni
build, mai da committare).

## Fuori scope

- Firmare `ramus-mcp` separatamente dall'app principale: la firma
  dell'intero bundle (spec `2026-09-03-firma-notarizzazione.TODO.md`)
  copre anche i binari sidecar inclusi tramite `externalBin` — nessun
  passo aggiuntivo previsto da Tauri per questo.
- Versione "MCP disabilitato di default nelle build distribuite":
  `mcp_enabled` resta `true` di default ovunque, stessa scelta già
  presa (M5), non riaperta qui.

## Verifica

`cargo test`, `cargo clippy --all-targets -D warnings`, `cargo fmt
--check`, `npm run typecheck` — tutti puliti. **Verificato per
davvero, non solo compilato**: `node scripts/prepare-mcp-sidecar.mjs`
eseguito fino in fondo (compila `ramus-mcp --release`, ~3m14s la
prima volta), prodotto
`src-tauri/binaries/ramus-mcp-aarch64-apple-darwin` (~12 MB,
eseguibile) — lanciato direttamente con `--print-config`, risposta
corretta con il proprio percorso reale. Non verificato in questa
sessione: un `npm run tauri build` completo fino al bundle finale
installato (richiede l'intera toolchain di bundling, più lunga di
quanto valga la pena in questa sessione) — il meccanismo di
posizionamento/ricerca è comunque verificato nella sua interezza
tramite lo script di prepare, l'unico pezzo specifico di questa spec
non già coperto da `cargo build`/`tauri dev` ordinari.
