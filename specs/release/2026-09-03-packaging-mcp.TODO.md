# Pacchettizzare `ramus-mcp` insieme all'app distribuita

Stato: proposta, da implementare. Nessuna domanda bloccante — una
sola decisione tecnica, proposta sotto con motivazione, non un
bivio di prodotto.

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

- `tauri.conf.json` → `bundle.externalBin: ["binaries/ramus-mcp"]` (
  convenzione Tauri: un file per target triple, generato da uno script
  di build prima del bundling — `cargo build -p ramus-mcp --release`
  seguito da una copia/rename nella cartella `binaries/`).
- Nuovo script (`src-tauri/build.rs` esteso, o uno script npm
  `prebuild:mcp`) che compila `ramus-mcp` e lo posiziona con il nome
  atteso da Tauri prima di `tauri build`.
- `find_mcp_binary()` (`commands.rs`): aggiunto un secondo tentativo,
  **dopo** quello attuale (sibling file — resta valido per `tauri
  dev`), che cerca nella cartella `resources`/sidecar della build
  pacchettizzata (percorso ottenuto da
  `tauri::path::PathResolver::resource_dir()`, già usato altrove per
  path platform-specific — vedi M6). Nessuna rimozione del percorso
  esistente: due tentativi in sequenza, non un'alternativa esclusiva.
- `get_mcp_info` (già esistente): nessuna modifica, continua a
  riflettere `binary_found` in base al risultato di `find_mcp_binary`.

## Fuori scope

- Firmare `ramus-mcp` separatamente dall'app principale: la firma
  dell'intero bundle (spec `2026-09-03-firma-notarizzazione.TODO.md`)
  copre anche i binari sidecar inclusi tramite `externalBin` — nessun
  passo aggiuntivo previsto da Tauri per questo.
- Versione "MCP disabilitato di default nelle build distribuite":
  `mcp_enabled` resta `true` di default ovunque, stessa scelta già
  presa (M5), non riaperta qui.

## Verifica

`cargo build -p ramus-mcp --release`, script di copia, poi `npm run
tauri build` — verificare che il bundle risultante contenga
`ramus-mcp` e che `get_mcp_info` lo trovi lanciando l'app **non** da
`tauri dev`, ma dal bundle installato (l'unico modo reale di
verificare questa spec: il bug che risolve esiste solo fuori
dall'ambiente di sviluppo).
