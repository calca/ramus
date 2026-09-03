# Impatti del supporto mobile sulle milestone M1-M5

Stato: proposta, in attesa di conferma. Non una spec da implementare
di per sé — un audit, richiesto esplicitamente insieme all'apertura
di M6, per catalogare cosa nelle spec già scritte (implementate o
no) assume un desktop e va rivisto quando si costruisce il mobile.
Ogni punto rimanda alla spec di origine; nessuno viene risolto qui.

## M1 — Journal funzionante (completa su desktop)

- **Selettore cartella vault** (`specs/M1/2026-09-02-settings.DONE.md`,
  bottone "Cambia"): non disponibile su mobile (verificato,
  `specs/M6/2026-09-03-supporto-mobile-fondamenta.TODO.md`). Il
  bottone va nascosto su build mobile, non semplicemente lasciato lì
  a fallire.
- **`Config::default_vault_path`/`config_file_path`**: dipendono da
  `dirs`, Android non è affidabile (verificato). Fix architetturale
  già descritto nella spec delle fondamenta — impatto diretto sul
  codice scritto in M1, non solo sulla UI.
- **Dimensioni finestra e modalità compatta**
  (`specs/M1/2026-09-02-dimensioni-finestra.DONE.md`): `setSize`/
  `setPosition`/dimensione minima/toggle compatto sono concetti di
  finestra desktop — su mobile l'app occupa sempre tutto lo schermo,
  nessuna finestra da ridimensionare o affiancare. Il bottone
  compact-toggle e la relativa logica vanno esclusi dalla build
  mobile (`#[cfg(desktop)]` lato Rust — stessa convenzione di cfg
  alias già usata nei plugin ufficiali Tauri, verificata in
  `tauri-plugin-dialog`; lato frontend, un controllo sulla
  piattaforma per non montare il bottone).
- **Overlay titlebar macOS**
  (`specs/M1/2026-09-02-overlay-titlebar-macos.DONE.md`): il padding
  riservato ai pallini semaforo (`padding-left: 84px` in
  `.app-header`) è specifico di macOS desktop — su mobile non c'è
  nessun pallino, quel padding diventa spazio sprecato in alto.
  Condizionale sulla piattaforma, stesso principio del punto sopra.
- **File watcher**: funziona su Android (nativo), su iOS ricade su
  polling (verificato) — non bloccante ma da tenere a mente per
  l'intervallo di polling scelto.
- **Rollover automatico a mezzanotte**
  (`specs/M1/2026-09-02-nuovo-giorno-automatico.DONE.md`): il
  meccanismo implementato (focus/visibilitychange +
  polling 60s) va verificato su mobile — `visibilitychange` esiste
  nelle webview mobile, ma un'app iOS in background viene sospesa
  (il `setInterval` non ticka finché non torna in foreground): non è
  un problema, `visibilitychange` al ritorno in foreground copre
  comunque il caso, probabilmente **meglio** su mobile che su
  desktop (dove un desktop può restare a fuoco ininterrottamente più
  facilmente).

## M2 — Link e ricerca (completa su desktop)

- **SQLite (`rusqlite`, bundled)**: nessun problema atteso, SQLite
  bundled/vendored si compila di routine per target mobile — non
  verificato con una build reale in questo sandbox, ma non è un
  rischio noto.
- **tantivy**: **da verificare**, non dato per scontato. È una
  dipendenza pesante con componenti nativi (mmap, eventualmente
  simd/codec specifici) — non ho verificato in questo sandbox se
  compila pulito per `aarch64-linux-android`/`aarch64-apple-ios`.
  Prima cosa da controllare quando si passa dalla progettazione alla
  build reale, prima di scrivere altro codice mobile che ne dipende.
- **Autocomplete `[[`/`#`** (`specs/M2/2026-09-02-link-tag-parsing.DONE.md`,
  `specs/M2/2026-09-02-autocomplete-tag.DONE.md`): popup posizionato
  via `clientRect()`, dovrebbe funzionare in una webview mobile senza
  modifiche — l'interazione touch con la tastiera a schermo per
  digitare `[[`/`#` e poi navigare il popup con testo (niente frecce
  fisiche) va verificata a mano, non è chiaro se serva un adattamento
  finché non si prova.

## M3 — Git (nessun pezzo implementato, entrambe le spec TODO)

Impatto più grande di tutti gli altri messi insieme — vale la pena
saperlo **prima** di implementare M3, non dopo:

- **Autenticazione remota** (`specs/M3/2026-09-02-sync-git-remoto.TODO.md`):
  la spec assume `Cred::ssh_key_from_agent`/`Cred::credential_helper`
  di `git2` — cioè un agente SSH di sistema o un credential helper
  desktop (Keychain macOS, Credential Manager Windows). **Nessuno dei
  due esiste allo stesso modo su Android/iOS.** Serve un meccanismo
  di credenziali completamente diverso per mobile (es. token
  personale salvato in Keychain iOS / Android Keystore tramite un
  plugin dedicato) — non un adattamento minore, una spec mobile a
  parte quando si arriva a costruire M3 lì.
- **Timer di sync in background**
  (`specs/M3/2026-09-02-sync-git-locale.TODO.md`, task
  `tauri::async_runtime::spawn` con `tokio::time::interval`): su
  desktop funziona perché il processo resta vivo. **Su mobile, e
  specialmente su iOS, il sistema operativo sospende l'esecuzione in
  background quasi subito** — un timer in-process semplicemente non
  ticka quando l'app non è in primo piano. Serve l'integrazione con le
  API di background task del sistema (iOS `BGTaskScheduler`, Android
  `WorkManager`) per qualunque sync che debba avvenire senza che
  l'utente abbia l'app aperta — infrastruttura che Tauri non fornisce
  pronta all'uso, andrebbe verificato se esiste un plugin community o
  va scritta ad hoc.

**Conseguenza pratica**: quando si arriva a implementare M3, vale la
pena valutare se il mobile lo ottiene più avanti, dopo il desktop, con
una spec propria — non è un "lo stesso codice gira anche lì", è
sostanzialmente un secondo pezzo da progettare.

## M4 — UI (nessun pezzo implementato, tutte le spec TODO)

- **Scorciatoie da tastiera** (`specs/M4/2026-09-02-scorciatoie-configurabili.TODO.md`,
  `specs/M4/2026-09-02-riordino-blocchi-tastiera.TODO.md`,
  `specs/M4/2026-09-02-focus-mode-navigazione-giorni.TODO.md`):
  l'intera idea "keyboard focused" presuppone una tastiera fisica.
  Senza una collegata via Bluetooth (possibile ma non il caso comune
  su telefono), Cmd/Ctrl+K, Alt+Su/Giù, Mod+. eccetera semplicemente
  non sono raggiungibili. Su mobile servono equivalenti touch (bottoni
  visibili, drag handle per riordinare, un FAB per la command
  palette) — non la stessa UI con un trigger diverso, un'interazione
  diversa da progettare a parte. La sezione "Scorciatoie" in
  Impostazioni (cattura-tasto) non ha senso su mobile, va nascosta.
- **Command palette** (`specs/M4/2026-09-02-command-palette.TODO.md`):
  la logica (ricerca, crea pagina, recenti, azioni) resta valida —
  serve solo un modo di **aprirla** senza Cmd/Ctrl+K (es. un bottone
  nell'header/status bar, sempre visibile su mobile invece che dietro
  una scorciatoia).
- **Header compatto + status bar**
  (`specs/M4/2026-09-02-header-status-bar.TODO.md`): il layout
  proposto (3 icone in alto, nav in basso) è già abbastanza vicino a
  un pattern mobile-friendly — l'adattamento principale è il rispetto
  delle safe area (notch, home indicator) via CSS
  `env(safe-area-inset-*)`, non una riprogettazione.
- **Task nei blocchi** (`specs/M4/2026-09-02-task-todo-done.TODO.md`):
  il click sul marker per il toggle si traduce in un tap senza
  modifiche — nessun impatto. La scorciatoia `Mod-Enter` per il ciclo
  a tre stati resta un problema "tastiera fisica" come sopra, ma qui
  esiste già un'alternativa touch naturale (il tap sul marker copre
  già il caso più comune, fatto/da fare).

## M5 — AI → server MCP (nessun pezzo implementato, entrambe le spec TODO)

- **Non si applica al mobile**, non per una lacuna da colmare ma per
  natura dell'architettura: `ramus-mcp`
  (`specs/M5/2026-09-02-mcp-server-lettura.TODO.md`,
  `specs/M5/2026-09-02-mcp-server-scrittura.TODO.md`) è un binario a
  sé, avviato via stdio da un client MCP desktop (Claude Desktop,
  Claude Code) — un modello a processi che il sandboxing mobile non
  permette, e i client MCP stessi girano su desktop. **M5 resta
  desktop-only per costruzione**, non serve una versione mobile: va
  solo scritto esplicitamente da qualche parte (qui) perché non venga
  dato per scontato più avanti.

## Non-impatti (verificati, nessuna azione richiesta)

Per completezza — cose che sembravano a rischio ma non lo sono, dopo
verifica:

- Round-trip del parser, modello a blocchi, formato su disco: zero
  dipendenza dalla piattaforma, `ramus-core` è puro Rust già portabile
  per costruzione (CLAUDE.md regola 1).
- Indice SQLite (M2): nessun problema noto per mobile.

## Decisione: M6 non si anticipa, tranne un pezzo

Deciso: M6 resta dov'è nell'ordine (dopo M5), **tranne** il refactor
di `Config::default_vault_path`/`config_file_path` (path iniettato dal
chiamante invece di calcolato con `dirs` — descritto in
`specs/M6/2026-09-03-supporto-mobile-fondamenta.TODO.md`, sezione 1),
che si fa **prima di iniziare M3**. Motivo: è piccolo, migliora
l'architettura anche per il solo desktop, e più codice si scrive
contro la firma attuale di quelle funzioni più costa cambiarle dopo —
tutto il resto del lavoro mobile (init Tauri, credenziali via
keychain, varianti touch, background task) resta a valle di M3/M4/M5
com'era, non ha senso costruirlo prima di aver fissato l'UX desktop di
quelle feature.

Le due domande originarie (ordine di lavoro completo, se aspettare
M3/M4 finché il mobile non è più chiaro) sono risolte da questa
decisione: nessuna attesa, si procede su desktop, si accetta di
ritoccare quando si arriva al mobile — eccetto il refactor dei path,
che precede tutto.

## Verifica

Nessuna — è un documento di analisi, non una spec da implementare.
Le voci useranno la Verifica di ciascuna spec di origine quando quella
verrà effettivamente costruita per mobile.
