# Overlay title bar (macOS)

Stato: implementata e verificata a vista su macOS (screenshot dell'utente).
Le API sono verificate direttamente nei sorgenti di `tauri-utils 2.9.3`
(la versione già in uso, vedi `Cargo.lock`), non da documentazione che
potrebbe riferirsi ad un'altra versione.

## Motivazione

Oggi la finestra usa la barra del titolo nativa di default (`titleBarStyle`
non impostato = `Visible`). Su macOS si passa alla modalità overlay: la
barra nativa sparisce, restano solo i tre pallini (traffic light) che
galleggiano sopra il contenuto, e l'`app-header` che abbiamo già (logo,
titolo, controlli journal, ingranaggio impostazioni) diventa di fatto la
barra del titolo. È lo stesso trattamento di Arc/Notion/Linear, coerente
con l'estetica "sobria" di SPEC.md — non richiede disegnare a mano i
pulsanti di finestra (restano nativi, disegnati dal sistema).

Riguarda solo macOS: `titleBarStyle`/`trafficLightPosition`/`hiddenTitle`
sono campi specifici di macOS nella struct di configurazione di Tauri,
ignorati sulle altre piattaforme. Su Windows e Linux la finestra resta
com'è oggi (barra nativa piena), **zero modifiche, zero codice
condizionale per piattaforma**: è già così per costruzione, non c'è
niente da implementare per il fallback.

## Configurazione (`src-tauri/tauri.conf.json`)

```json
{
  "app": {
    "windows": [
      {
        "title": "Ramus",
        "width": 800,
        "height": 600,
        "titleBarStyle": "Overlay",
        "hiddenTitle": true
      }
    ]
  }
}
```

- `titleBarStyle: "Overlay"`: barra nativa trasparente sopra il
  contenuto, solo i pallini restano. Richiede `decorations: true` (è già
  il default, non toccarlo).
- `hiddenTitle: true`: nasconde il testo del titolo nativo — non serve,
  "Ramus" è già mostrato nel nostro `app-header` via logo + testo.
- `title`: corretto in maiuscolo (`"Ramus"`, era `"ramus"`) — usato per
  dock/task switcher, cosmetico, non legato all'overlay in sé.
- **`trafficLightPosition` non impostato inizialmente**: lascia la
  posizione di default del sistema. Se dopo aver visto il risultato reale
  serve allinearla meglio con l'altezza del nostro header, si aggiunge
  `"trafficLightPosition": { "x": ..., "y": ... }` (coordinate logiche) —
  va tarata a vista, nessun valore corretto a priori.

## Frontend

`app-header` (in `App.tsx`) deve diventare l'area di trascinamento della
finestra, dato che con la barra nativa nascosta quel comportamento non è
più automatico:

- Aggiungere l'attributo `data-tauri-drag-region` al contenitore
  `app-header`.
- **Permesso mancante scoperto durante il test**: `core:default` (già
  in `capabilities/default.json`) include `allow-internal-toggle-maximize`
  (doppio click) ma **non** `allow-start-dragging` — senza
  `"core:window:allow-start-dragging"` esplicito nei permessi, il drag
  vero e proprio viene bloccato silenziosamente dal sistema di capability
  e la finestra non si sposta, anche con l'attributo HTML corretto.
- **Non serve nessun accorgimento per i bottoni/input già dentro
  l'header** (i controlli journal, l'ingranaggio impostazioni, il date
  picker): verificato nello script di drag di Tauri
  (`tauri/src/window/scripts/drag.js`) che gli elementi cliccabili
  (`button`, `input`, `a`, elementi con `role` interattivo, ecc.) senza
  l'attributo esplicito **bloccano automaticamente il trascinamento** su
  se stessi — restano cliccabili normalmente, il drag si attiva solo
  sullo sfondo dell'header. Nessuna modifica ai componenti esistenti.
- Aggiungere `padding-left` all'`app-header` per lasciare spazio ai
  pallini (che ora si sovrappongono al contenuto invece di stare in una
  barra separata). Confermato a vista: `84px`.

## Limitazioni note (upstream Tauri, non risolvibili da noi)

Dai commenti della stessa API `TitleBarStyle::Overlay`:

- L'altezza della barra overlay varia fra versioni di macOS: i pallini
  potrebbero non essere esattamente dove ci si aspetta su OS diversi.
  Non c'è una soluzione lato nostro, solo tenerne conto.
- Non si può trascinare la finestra quando non è a fuoco (limite noto di
  Tauri, [tauri-apps/tauri#4316](https://github.com/tauri-apps/tauri/issues/4316)).
  Comportamento accettato, non è nel nostro controllo.

## Fuori scope per questa spec

- Effetto vibrancy/vetro smerigliato dietro l'header (macOS supporta
  materiali vibrancy, ma è una dipendenza/API separata da valutare a
  parte, non necessaria per il solo overlay dei pallini).
- Barra del titolo completamente custom (`decorations: false`, pulsanti
  di finestra disegnati a mano): scartata nella discussione iniziale a
  favore di questa opzione, più semplice e meno rischiosa.
- Qualunque modifica per Windows/Linux: non serve, vedi sopra.

## Verifica

Non testabile con `cargo test`/`npm run typecheck` (è comportamento
visivo/di finestra). Checklist manuale in `npm run tauri dev` su macOS:

- I tre pallini sono visibili e funzionano (chiudi/minimizza/massimizza).
- Il contenuto dell'header non è coperto dai pallini.
- Si può trascinare la finestra cliccando sullo sfondo dell'header.
- I controlli dentro l'header (Oggi, date picker, ingranaggio) restano
  cliccabili e **non** iniziano un trascinamento quando ci si clicca sopra.
- Doppio click sullo sfondo dell'header massimizza la finestra (comportamento
  di sistema, viene gratis dallo script di drag di Tauri).
