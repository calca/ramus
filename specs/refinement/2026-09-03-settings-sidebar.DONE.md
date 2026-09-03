# Impostazioni: layout a sidebar invece di scroll unico

Stato: implementata. Prima spec della fase di refinement (vedi
`SPEC.md`, "Fase 1 completa" — non appartiene a nessuna milestone
specifica, tocca il layout del pannello Impostazioni nel suo
complesso).

## Motivazione

`SettingsPanel` oggi concatena sei sezioni (Vault, Tema, Scorciatoie,
Task, MCP, Sync) più il link "Informazioni su Ramus" in un'unica
colonna scrollabile (`max-height: calc(100vh - 4rem)`). Su schermi
piccoli, o semplicemente con la lista scorciatoie espansa, arrivare a
Sync richiede uno scroll lungo — segnalato dall'utente come "troppo
affollato per una pagina".

## Layout proposto

Sidebar di categorie a sinistra, contenuto della categoria selezionata
a destra — stesso pattern delle impostazioni di macOS/VS Code:

```
┌─────────────────────────────────────────────┐
│ Impostazioni                             ✕   │  <- header, invariato
├───────────────┬───────────────────────────────┤
│ Vault         │                               │
│ Tema       │   (contenuto della sezione     │
│ Scorciatoie   │    selezionata, scroll solo    │
│ Task          │    qui se serve)               │
│ MCP           │                               │
│ Sync          │                               │
├───────────────┤                               │
│ Informazioni  │                               │
└───────────────┴───────────────────────────────┘
```

- Solo la sezione attiva è montata nel pannello destro: niente scroll
  a meno che quella singola sezione sia lunga (es. Scorciatoie con
  tutti i comandi elencati).
- "Informazioni su Ramus" resta un bottone che apre `AboutPanel` come
  modal separato (comportamento invariato) — messo in fondo alla
  sidebar, staccato dalla lista delle categorie da un separatore, per
  distinguerlo visivamente: non è una categoria con contenuto inline,
  è un'azione che apre un altro pannello.
- Prima categoria selezionata di default: **Vault** (la prima della
  lista attuale, nessuna preferenza persistita — riaprire Impostazioni
  parte sempre da lì, stesso principio di semplicità già seguito per
  gli altri toggle di default).

## Modifiche

**`SettingsPanel.tsx`**:
- Nuovo stato locale `activeSection: SettingsSectionId` (default
  `"vault"`), un tipo unione delle sei categorie.
- Il JSX di ogni sezione (`<section className="settings-section">`)
  resta identico nel contenuto — handler, stato, chiamate ai command
  tutte invariate — ma solo la sezione con `id === activeSection`
  viene renderizzata, invece di tutte in sequenza.
- Nuovo elemento `<nav className="settings-sidebar">` con un bottone
  per categoria (`aria-current="true"` su quello attivo, stesso
  pattern di accessibilità già in uso per i toggle esistenti — niente
  libreria nuova, solo `<button>` con class condizionale).
- Il body del modal diventa un contenitore flex-row
  (`.settings-body`) con due figli: `.settings-sidebar` e
  `.settings-content`; l'header (titolo + ✕) resta sopra, invariato,
  fuori da questo contenitore.

**`index.css`**:
- `.settings-panel`: larghezza da `min(28rem, calc(100vw - 2rem))` a
  `min(38rem, calc(100vw - 2rem))` (serve spazio per la sidebar);
  `max-height` invariato.
- Nuove regole `.settings-body` (flex row, `align-items: stretch`),
  `.settings-sidebar` (colonna di bottoni, larghezza fissa ~9rem,
  bordo destro sottile per separarla dal contenuto),
  `.settings-sidebar button` (stile bottone piatto, stato attivo
  evidenziato con lo stesso colore d'accento già usato altrove —
  niente palette nuova, solo variabili esistenti di
  `assets/palette.css`), `.settings-content` (flex: 1, `overflow-y:
  auto`, padding proprio invece di ereditare quello del pannello).
- `.settings-section` perde `margin-top` (non più sezioni impilate,
  ognuna occupa già tutto `.settings-content`).

**Nessuna modifica** a nessun handler, command Tauri, o tipo — è un
refactor di layout puro, zero impatto su Rust o sulla logica.

## Fuori scope per questa spec

- Ricordare l'ultima categoria aperta fra un'apertura e l'altra del
  pannello (localStorage o campo in `Config`): nessuna richiesta in
  merito, si può aggiungere in una spec a parte se servisse.
- Ricerca testuale fra le impostazioni (come il "cerca" di VS Code
  Settings): fuori scope, sei categorie non lo giustificano.
- Qualunque cambiamento al contenuto delle sezioni stesse (nuove
  opzioni, testi diversi): questa spec tocca solo il contenitore.

## Domande aperte

Nessuna: layout, categorie e ordine ricalcano 1:1 le sezioni esistenti
(stesso ordine: Vault, Tema, Scorciatoie, Task, MCP, Sync), nessuna
decisione lasciata in sospeso.

## Test da scrivere

Nessun test nuovo lato Rust (zero modifiche a `ramus-core`/command).
Lato frontend, coerente con l'assenza di un runner JS per componenti
nel progetto (stessa scelta di tutte le altre spec di
`SettingsPanel`): verifica manuale via `npm run tauri dev`.

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — tutti puliti (zero modifiche Rust,
verificati comunque per la regola di CLAUDE.md). Non verificato in
questa sessione con uno screenshot reale: `npm run tauri dev` era già
in esecuzione (hot reload via Vite), ma non è stata scattata una
schermata della UI aggiornata — verifica visiva lasciata all'utente
nell'app già aperta.
