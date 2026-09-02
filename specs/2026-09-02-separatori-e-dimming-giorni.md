# Separatori fra giorni e dimming dei giorni senza focus

Stato: implementata. Il valore di opacity (`0.5`) resta da confermare a
vista — non verificabile in questo sandbox — come già successo per il
padding della title bar overlay.

## Motivazione

Nella vista journal verticale (vedi
`2026-09-02-journal-vista-verticale.md`) i giorni sono oggi separati solo
da uno spazio bianco (`margin-bottom`), senza una linea visibile — con
molte sezioni caricate è difficile individuare a colpo d'occhio dove
finisce un giorno e comincia il successivo, e su quale si sta
effettivamente scrivendo. Due correzioni, entrambe di stile puro,
nessuna modifica al modello dati o ai command:

1. Un separatore visibile fra un giorno e il successivo.
2. I giorni che non hanno il focus dell'editor si affievoliscono
   (dimmed), per far risaltare quello su cui si sta scrivendo.

## Separatore fra giorni

CSS-only, nessuna modifica ai componenti React. In `index.css`, un
bordo superiore sulla sezione successiva invece di margine "vuoto" fra
le due — semantica più precisa di "separatore fra", non "bordo su ogni
sezione":

```css
.journal-section {
  max-width: 40rem;
  margin: 0 auto;
  padding-bottom: 2.5rem;
}

.journal-section + .journal-section {
  border-top: 1px solid color-mix(in srgb, var(--ramus-stone) 20%, transparent);
  padding-top: 2rem;
}
```

(Sostituisce l'attuale `.journal-section { margin: 0 auto 2.5rem; }`.)
Stesso stone-a-bassa-opacità già usato per il bordo sotto `app-header`,
coerente con la palette esistente.

## Dimming dei giorni senza focus

Approccio CSS-only con `:focus-within`, **nessuno stato in `App.tsx`,
nessuna modifica a `Editor.tsx`/`JournalSection.tsx`**: ogni
`.journal-section` riceve automaticamente `:focus-within` quando il
cursore è nel suo editor Tiptap (un div `contenteditable` dentro la
sezione — `:focus-within` matcha nativamente, non serve tracciare nulla
via JS).

Il problema da risolvere: affievolire gli *altri* giorni solo quando
*qualcuno* ha il focus — altrimenti al primo caricamento, prima di aver
cliccato in un blocco, ogni sezione risulterebbe spenta (nessuna ha
focus). Serve `:has()` sul contenitore per condizionare la regola
all'esistenza di *una* sezione a fuoco:

```css
.app-body:has(.journal-section:focus-within) .journal-section:not(:focus-within) {
  opacity: 0.5;
  transition: opacity 150ms ease;
}
```

Comportamento risultante:
- Nessun editor a fuoco (avvio, o dopo aver cliccato fuori da ogni
  editor — date picker, bottone Oggi, ingranaggio impostazioni) → tutte
  le sezioni a piena opacità.
- Si clicca/scrive in un blocco → quella sezione resta piena, tutte le
  altre si affievoliscono con una transizione morbida (150ms).
- Si sposta il cursore su un altro giorno → il dimming si aggiorna da
  solo (nuova sezione piena, la precedente si affievolisce), sempre
  senza codice: è la cascata di `:focus-within` che cambia.

### Compatibilità di `:has()`

Richiede un motore di rendering abbastanza recente (WebKit 15.4+,
Chromium 105+, Firefox 121+). Sulle piattaforme target di Tauri
(WKWebView su macOS, WebView2 su Windows, WebKitGTK su Linux) dovrebbe
essere ampiamente coperto, ma su una WebKitGTK datata di qualche distro
Linux la regola verrebbe semplicemente **ignorata** dal motore — nessun
errore, il dimming non si attiva, il resto dell'app funziona
normalmente. Degradazione morbida, accettabile per un effetto puramente
cosmetico.

Se in pratica risultasse un problema reale (da verificare, non
presumibile a priori): alternativa è spostare la logica in `App.tsx` con
`onFocus`/`onBlur` su `Editor.tsx` e uno stato `focusedPath`, applicando
una classe invece del selettore CSS. Più codice, compatibilità totale.
Si valuta solo se `:has()` si rivela davvero un problema, non
preventivamente.

## Valore di opacity

Proposta `0.5` per le sezioni non a fuoco — stima di partenza, non
verificabile a vista in questo sandbox (nessun accesso a
screen-recording). Da confermare/correggere guardando l'app, come già
fatto per il padding della title bar overlay.

## Font leggermente più grande sul giorno a fuoco

Aggiunta dopo il primo giro di implementazione, stesso meccanismo
`:focus-within` (nessun nuovo stato):

```css
.journal-section:focus-within .ramus-editor {
  font-size: 1.05em;
  transition: font-size 150ms ease;
}
```

Rinforza l'enfasi del dimming: il giorno su cui si scrive non solo
resta a piena opacità, ma si legge un filo più grande delle altre
sezioni (affievolite). L'header della data (`.journal-section-date`) è
in `rem`, non scala con questo — resta della stessa dimensione,
volutamente: solo il testo dei blocchi cresce.

## Fuori scope

- Dimming basato sullo scroll/sezione più visibile in viewport (diverso
  da focus dell'editor, richiederebbe un `IntersectionObserver`): non
  richiesto, più complesso, lasciato fuori.
- Colore/stile diverso per il separatore sopra la sezione di oggi
  rispetto agli altri: non richiesto — "oggi" resta già distinto
  dall'header ambrato esistente (`.journal-section-date-today`).

## Verifica

Non testabile con `cargo test`/`npm run typecheck` (comportamento
visivo/CSS puro, nessun file `.ts`/`.rs` toccato). Checklist manuale in
`npm run tauri dev`:

- Linea sottile visibile fra un giorno e il successivo, non sopra il
  primo (oggi).
- All'avvio, prima di cliccare in un blocco, nessuna sezione è
  affievolita.
- Cliccando/scrivendo in un blocco di un giorno, tutti gli altri giorni
  si affievoliscono con una transizione morbida, non uno scatto.
- Spostando il cursore su un altro giorno, il dimming si aggiorna da
  solo sulla nuova sezione.
- Cliccando fuori da ogni editor (date picker, bottone Oggi, ingranaggio),
  tutte le sezioni tornano a piena opacità.
