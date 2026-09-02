# Dimensioni finestra: minimo + modalità compatta

Stato: implementata. Nessun riposizionamento automatico, nessuna
persistenza. 420/480px restano stime di partenza — non verificabili a
vista in questo sandbox, da tarare guardando l'app affiancata a una
finestra di note reale.

## Motivazione

Due richieste distinte:

1. Una dimensione minima della finestra, sotto la quale non si può
   ridimensionare (oggi non c'è alcun vincolo).
2. Un bottone in header che passa la finestra a una larghezza stretta,
   per tenerla affiancata a destra di una finestra più grande (note,
   appunti) — un compagno di scrittura laterale, non la finestra
   principale a schermo pieno.

## Proposta

### Dimensione minima

```json
{
  "app": {
    "windows": [
      {
        "minWidth": 420,
        "minHeight": 560
      }
    ]
  }
}
```

Campi verificati nei sorgenti di `tauri-utils 2.9.3` (`min_width`,
`min_height` nella struct di config, camelCase in JSON, coerente col
resto di `tauri.conf.json`). `360`/`480` iniziali risultati troppo
piccoli all'uso; alzati a `420`/`560`. `minWidth` combacia esattamente
con `COMPACT_WIDTH` (420, vedi sotto): il minimo assoluto della
finestra è la larghezza compatta stessa, non qualcosa di ancora più
stretto — la modalità compatta diventa "il pavimento", non un valore
sotto al pavimento.

### Modalità compatta

Bottone testuale in `app-header`, accanto all'ingranaggio impostazioni
(stesso stile degli altri bottoni di testo — "Oggi", "Cambia" — non
un'icona, coerente con la UI esistente): **"Compatta"** quando la
finestra è normale, **"Espandi"** quando è compatta.

Comportamento:
- Al primo "Compatta": si memorizza la dimensione attuale della
  finestra (larghezza e altezza logiche, non fisiche — serve
  `scaleFactor()` per convertire), poi si ridimensiona a
  `{ width: 420, height: <altezza attuale, invariata> }`.
- **Solo la larghezza cambia**, l'altezza resta quella che l'utente
  aveva già impostato — "compatta" è "stretta", non "piccola".
- A "Espandi": si ripristina la dimensione memorizzata (non un valore
  fisso) — se l'utente aveva ridimensionato la finestra a 1000×700
  prima di comprimerla, torna a 1000×700, non al default 800×600.
- **Nessun riposizionamento automatico** (non si sposta la finestra
  contro il bordo destro dello schermo da sola): l'utente la trascina
  dove vuole — ora è facile, l'`app-header` ha già
  `data-tauri-drag-region` dalla spec della title bar overlay. Scelta
  deliberata per restare semplice e non presumere un layout schermo
  che potremmo indovinare male (multi-monitor, split view nativo di
  macOS, ecc.) — vedi "Domande aperte" se invece si vuole automatico.
- **Non persistita**: si torna sempre alla modalità normale a ogni
  riavvio dell'app. È una preferenza di sessione di lavoro (affianca
  Ramus a un'altra finestra per un po'), non una configurazione
  permanente — coerente con "zero attrito all'avvio" di SPEC.md: non
  si vuole che l'app si apra compatta per errore e sembri rotta. Se
  serve persisterla, è un'aggiunta a `Config` da proporre a parte.

### API

```ts
import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";

const win = getCurrentWindow();
const physical = await win.innerSize();
const scale = await win.scaleFactor();
const current = physical.toLogical(scale); // { width, height }

await win.setSize(new LogicalSize(420, current.height));
```

Verificato in `node_modules/@tauri-apps/api/{window,dpi}.js`:
`innerSize()`/`scaleFactor()`/`toLogical()` esistono con questa firma,
`setSize` accetta un `LogicalSize`. Nessun nuovo command Tauri, nessuna
modifica al core: è tutto frontend, stato locale in `App.tsx` (un
`useState`/`useRef` per larghezza+altezza pre-compattazione).

### Header responsive sotto ai 480px

A 420px di larghezza, il contenuto attuale dell'header (logo, titolo,
controlli journal, toggle compatta, ingranaggio) non ci sta tutto
comodamente — soprattutto coi 84px di padding-left riservati ai
pallini macOS. Invece di nascondere elementi via JS in base allo stato
"compatto", una media query CSS chiave sulla larghezza reale della
finestra: si applica sia al toggle sia a un ridimensionamento manuale
(es. split view nativo di macOS), un solo meccanismo invece di due:

```css
@media (max-width: 480px) {
  .app-title {
    display: none;
  }
  .journal-controls button:not(.settings-button) {
    /* "Oggi" diventa un'icona invece di testo, o si nasconde il
       date-picker — dettaglio da tarare a vista */
  }
}
```

Il dettaglio esatto di cosa nascondere/restringere a questa larghezza
va tarato a vista (come già successo per il padding della title bar) —
qui la spec fissa solo il meccanismo (media query sulla larghezza,
non JS legato allo stato del toggle) e la soglia (480px, sopra i 420px
della modalità compatta con un po' di margine).

## Domande aperte

- Il valore `420px` per la larghezza compatta e `480px` per la soglia
  della media query sono proposte di partenza — non verificabili a
  vista in questo sandbox, da confermare/tarare guardando l'app
  affiancata a una finestra di note reale.
- Riposizionamento automatico contro il bordo destro dello schermo
  all'attivazione: proposto **di no** (vedi sopra). Se invece lo si
  vuole, si può aggiungere con `currentMonitor()` +
  `setPosition()` — cambio piccolo, ma va deciso esplicitamente perché
  è un comportamento più invasivo (sposta la finestra senza che
  l'utente l'abbia chiesto).
- Persistenza della modalità compatta fra riavvii: proposto di no (vedi
  sopra). Se serve, è un campo nuovo in `Config` (stesso pattern di
  `theme`), da trattare come una spec a parte per non mischiare stato
  di sessione con configurazione permanente.

### Bug scoperti dopo il primo giro di implementazione

1. **Il bottone non faceva nulla**: `allow-set-size` non è nel set di
   default della finestra (`core:default`) — stesso identico problema
   di `allow-start-dragging` nella spec della title bar overlay.
   `innerSize`/`scaleFactor` (lettura) sono coperti dal default,
   `setSize` (scrittura) no. Aggiunto `core:window:allow-set-size`
   esplicito in `capabilities/default.json`.
2. **Bottone testuale invece che icona**: sostituito con lo stesso
   stile icon-only dell'ingranaggio impostazioni (`⇔`), con
   `aria-label`/`title` che cambia fra "Comprimi finestra"/"Espandi
   finestra" invece del testo visibile fisso "Compatta"/"Espandi".

## Fuori scope

- Layout dell'editor/contenuto adattivo oltre l'header (il corpo della
  vista journal è già fluido — `max-width: 40rem` con contenuto che si
  restringe naturalmente, nessuna modifica necessaria lì).
- Snap/docking automatico contro un'altra finestra specifica (fuori
  portata di Tauri: non c'è modo di sapere dove sia "la finestra delle
  note" di un'altra app).

## Verifica

Non testabile con `cargo test` (nessun codice Rust). Da verificare con
`npm run typecheck` e a vista in `npm run tauri dev`:

- La finestra non si ridimensiona sotto 360×480 trascinando i bordi.
- "Compatta" restringe a 420px di larghezza mantenendo l'altezza
  corrente; "Espandi" torna alla dimensione precedente esatta, non a
  un default fisso.
- Sotto 480px di larghezza (sia da toggle sia da resize manuale)
  l'header resta leggibile e utilizzabile, senza contenuto tagliato o
  sovrapposto.
