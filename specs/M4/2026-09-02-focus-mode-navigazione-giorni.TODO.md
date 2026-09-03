# Focus mode + navigazione fra giorni da tastiera

Stato: proposta, in attesa di conferma. Presuppone
`specs/M4/2026-09-02-scorciatoie-configurabili.DONE.md` (le due
scorciatoie qui descritte entrano in quel registro).

## Motivazione

Quinto pezzo di M4, ultima parte dell'idea "keyboard focused, less
UI": una scorciatoia che nasconde tutta la chrome per scrivere senza
distrazioni, e due scorciatoie per muoversi fra i giorni del journal
senza toccare il date-picker col mouse.

## Focus mode

Nuovo stato in sessione `isFocusMode: boolean` (non persistito fra
riavvii — stesso trattamento già scelto per `isCompact` e per i
"recenti" della command palette: preferenza della sessione corrente,
non uno stato che ha senso salvare su disco).

Attivato dalla scorciatoia configurabile `focus_mode` (nuova voce in
`SHORTCUT_ACTIONS`, default proposto `Mod+.`), che fa da **toggle**:
la stessa combinazione entra ed esce dalla modalità.

Effetto: si aggiunge la classe `is-focus` a `.app` (stesso pattern già
in uso per `is-compact` sull'header). CSS nasconde `.app-header` e
`.app-statusbar` (quest'ultima introdotta da
`specs/M4/2026-09-02-header-status-bar.DONE.md`, già implementata):

```css
.app.is-focus .app-header,
.app.is-focus .app-statusbar {
  display: none;
}
```

Nessun'altra modifica: l'editor e il contenuto restano esattamente
dove sono, semplicemente occupano tutto lo spazio verticale liberato
(già garantito da `.app-body { flex: 1; }`, nessun CSS aggiuntivo
necessario lì). Compatibile con la modalità compatta (le due sono
indipendenti, si può avere una finestra stretta E senza chrome insieme
— nessun conflitto, nessuna gestione speciale richiesta).

## Navigazione fra giorni da tastiera

Due nuove voci nel registro configurabile: `journal_prev_day` (default
proposto `Mod+ArrowUp`) e `journal_next_day` (default proposto
`Mod+ArrowDown`) — "prev"/"next" nel senso della lista (verso l'alto =
giorno più recente, verso il basso = giorno meno recente, coerente con
l'ordine "oggi in cima" già stabilito in M1).

Attive solo quando `view.kind === "journal"` (stesso gate già usato
per `JournalControls`) — in `PageView` non hanno un giorno a cui
riferirsi.

### Tracciare "il giorno corrente"

Non esiste oggi un concetto esplicito di "giorno attivo" in `App.tsx`
al di là del CSS (`:focus-within` per il dimming, puramente visivo,
non letto da JS). Serve un piccolo stato gemello lato JS:

- Nuovo `focusedPath: string | null` in `App.tsx`, aggiornato da un
  listener `focusin`/`focusout` sugli elementi già tracciati in
  `sectionElements` (stesso ref esistente, usato oggi solo per
  `scrollIntoView` — si aggiunge un secondo uso, non una nuova mappa).
- Se `focusedPath` è `null` (nessun editor ha il fuoco, es. l'utente
  ha appena aperto l'app e non ha ancora cliccato/digitato da nessuna
  parte), le scorciatoie operano relative al primo giorno caricato
  (oggi).

### Comportamento

1. Trova l'indice di `focusedPath` (o del fallback) dentro `pages`.
2. `journal_prev_day`: se l'indice non è già 0, scrolla al giorno
   precedente (`scrollToPath`, esistente) e sposta il fuoco
   dell'editor lì.
3. `journal_next_day`: se serve un giorno non ancora caricato, prima
   `fetchNextBatch()` (stessa primitiva già usata da scroll infinito e
   "salta a data" — riusata, non duplicata), poi scrolla e sposta il
   fuoco.
4. Spostare il fuoco richiede che `Editor` esponga un modo di
   richiederlo dall'esterno: `EditorHandle` (oggi solo `{ flush }`)
   guadagna `focus: () => void` (chiama `editor.commands.focus()`
   internamente) — stesso `ref` già raccolto in `editorHandles`,
   nessuna nuova infrastruttura di comunicazione.

## Fuori scope per questa spec

- Navigazione fra giorni dentro `PageView` (non ha senso, non è una
  sequenza di giorni).
- Indicatore visivo permanente di "quale giorno ha il fuoco" oltre al
  dimming CSS già esistente (`:focus-within`) — resta invariato,
  questa spec aggiunge solo l'equivalente lato JS per le scorciatoie,
  non un secondo indicatore visivo.
- Uscire dal focus mode con un tasto diverso dalla stessa scorciatoia
  (es. Escape): la coerenza con `Modal` (che usa Escape per chiudere
  pannelli) potrebbe confondere se Escape facesse anche altro qui —
  solo il toggle esplicito.

## Domande aperte

1. Default proposti: `focus_mode` → `Mod+.`, `journal_prev_day` →
   `Mod+ArrowUp`, `journal_next_day` → `Mod+ArrowDown`. Vanno bene, o
   preferisci combinazioni diverse?
2. Focus mode disponibile anche mentre `PageView` è aperta (nasconde
   comunque header/status bar, la pagina resta): proposto sì, nessun
   motivo di limitarlo alla sola vista journal. Confermi?

## Test da scrivere

Nessuno lato core: tutta la logica è frontend (stato React, CSS,
un piccolo metodo aggiunto a `EditorHandle`). Nessun test frontend
nuovo, coerente con l'assenza di un runner JS per componenti nel
progetto.

## Verifica

`npm run typecheck` per la parte automatizzabile. Il resto
(toggle del focus mode, navigazione fra giorni con caricamento di
nuovi batch quando serve, il fuoco che segue correttamente) richiede
un giro manuale in `npm run tauri dev`.
