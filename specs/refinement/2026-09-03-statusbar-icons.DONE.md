# Status bar: icone compatte allineate a sinistra, gear sempre visibile

Stato: implementata. Quarta spec della fase di refinement, seguito di
`specs/refinement/2026-09-03-header-single-command-button.DONE.md`
(quel refactor aveva rimosso ⚙ dall'header assumendo che Impostazioni
restasse raggiungibile dal badge di sync in status bar — falso: il
badge è renderizzato solo quando `syncState` non è `"disabled"` né
`"idle"`, cioè **mai** finché l'utente non attiva la sync Git. Bug
segnalato dall'utente: "il settings deve essere una gear [sempre
visibile]").

## Stato attuale (per chiarezza)

`.app-statusbar` contiene, in ordine: `<JournalControls>` (input data
+ bottone testuale "Oggi") e, condizionale, il badge sync (⇄/⚠, solo
quando c'è stato da segnalare, apre Impostazioni al click).

## Decisioni confermate dall'utente

1. **Gear sempre visibile, badge sync separato** (non un'icona unica
   che si trasforma): un ⚙ fisso in status bar apre Impostazioni
   sempre, anche a sync disabilitata/idle. Il badge sync resta
   com'è — condizionale, apre anche lui Impostazioni.
2. **Il selettore "vai a data" resta, ma diventa icona compatta** (non
   eliminato): l'utente ha chiesto esplicitamente un sostituto invece
   di perdere la funzione.

## Modifiche

**"Oggi" — rimosso, non sostituito**: "Vai a oggi" esiste già come
azione della command palette (`paletteActions.ts`, invariata) ed è
raggiungibile dal bottone ⌘ nell'header o da `Cmd+K` — non serve un
secondo punto d'accesso diretto in status bar, coerente con "togliamo
... today e facciamo diventare command" (la palette è già quel
"command").

**`JournalControls.tsx`**: prop `onToday` e il bottone "Oggi" rimossi.
Resta solo l'`<input type="date">` (`onJumpToDate` invariato). Il
componente diventa un wrapper sottile attorno al solo selettore data —
tenuto com'è (non vale la pena inline-arlo in `App.tsx` per un solo
elemento).

**`App.tsx`**: chiamata a `<JournalControls>` non passa più `onToday`.
Nuovo bottone ⚙ sempre visibile (quando `config` è pronto, stesso
guard già usato per gli altri bottoni header/status bar):

```tsx
{config && (
  <button
    type="button"
    className="statusbar-icon-button"
    aria-label="Impostazioni"
    title="Impostazioni"
    onClick={() => setActivePanel("settings")}
  >
    ⚙
  </button>
)}
```

Ordine in `.app-statusbar` (tutto allineato a sinistra, nessuno
spacer — comportamento flex già presente, nessuna modifica di
allineamento serve): selettore data (icona) → ⚙ → badge sync
(condizionale).

**`index.css`**:
- Nuova classe condivisa `.statusbar-icon-button` (bordo sottile,
  sfondo trasparente, `color: var(--ramus-stone)`, padding
  `0.15rem 0.4rem`, `font-size: 0.85rem`) — più compatta del vecchio
  `.settings-button` dell'header (`padding: 0.25rem 0.55rem`,
  `font-size: 1rem`), per "ridurre la dimensione" richiesto. `.sync-badge`
  applicata insieme a questa classe nel JSX (`"statusbar-icon-button
  sync-badge"`, `+ is-conflict` quando serve) invece di duplicare le
  proprietà di base — `.sync-badge` resta per la sola variante colore
  di conflitto.
- `.journal-controls button` (regola per il vecchio bottone "Oggi",
  base e dentro `.app-statusbar`) rimossa: nessun bottone testuale
  resta dentro `.journal-controls`.
- `.journal-date-picker` ridotto a icona: niente più testo/cifre della
  data visibili, resta solo l'indicatore calendario nativo cliccabile
  (stessa area di click, stesso comportamento — apre il date-picker
  nativo del sistema, `onJumpToDate` invariato). Tecnica: nascondere le
  sotto-parti del controllo nativo con gli pseudo-elementi
  `::-webkit-datetime-edit*` (supportati sia da WebKit sia da Chromium,
  quindi sia da WKWebView su macOS sia da WebView2 su Windows — i due
  motori usati da Tauri) e restringere la larghezza a quella
  dell'icona:
  ```css
  .journal-date-picker {
    width: 1.6rem;
    padding: 0.15rem 0.3rem;
    /* ... bordo/sfondo invariati, coerenti con .statusbar-icon-button */
  }
  .journal-date-picker::-webkit-datetime-edit,
  .journal-date-picker::-webkit-datetime-edit-fields-wrapper,
  .journal-date-picker::-webkit-datetime-edit-text,
  .journal-date-picker::-webkit-datetime-edit-day-field,
  .journal-date-picker::-webkit-datetime-edit-month-field,
  .journal-date-picker::-webkit-datetime-edit-year-field {
    display: none;
  }
  ```
  **Fragilità nota**: pseudo-elementi non standard (prefisso
  `-webkit-`, non in nessuna spec W3C), ma implementati identicamente
  da entrambi i motori nativi di Tauri — rischio pratico basso essendo
  un runtime controllato (non il web aperto), non un vero cross-browser
  target. Se il rendering risultasse comunque ingombrante o l'icona
  nativa poco cliccabile in prova, il fallback più semplice è allargare
  leggermente `width` invece di tornare al testo completo.

## Fuori scope

- Aggiungere un bottone ⌘ duplicato in status bar: l'header ce l'ha
  già, un secondo punto d'accesso alla stessa azione non aggiunge
  nulla.
- Un modo di "saltare a data" testuale dentro la command palette
  (digitare una data nel campo di ricerca): cambierebbe il modello
  della palette (lista fissa di azioni + ricerca full-text, vedi
  commento in cima a `paletteActions.ts`) per un guadagno marginale
  rispetto a un'icona dedicata già funzionante — spec a parte se
  servisse davvero in futuro.
- Comportamento in modalità compatta: `.app-statusbar.is-compact {
  display: none; }` resta invariato (l'intera status bar sparisce,
  scelta già presa a M4, non in discussione qui).

## Domande aperte

Nessuna: entrambe le decisioni di design confermate dall'utente prima
di scrivere questa spec (vedi sopra). La tecnica di compattazione del
date-picker è una scelta implementativa (non di prodotto), documentata
con la sua fragilità nota invece che proposta come domanda.

## Test da scrivere

Nessuno, zero modifiche Rust/core. Coerente con l'assenza di un runner
JS per componenti nel progetto.

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — tutti puliti (zero modifiche Rust).
Verifica visiva lasciata all'utente in `npm run tauri dev` (hot
reload): in particolare il rendering reale dell'icona calendario
compattata, che non è verificabile da riga di comando.
