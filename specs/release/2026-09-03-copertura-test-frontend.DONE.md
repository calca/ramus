# Estendere la copertura di test del frontend

Stato: implementata (i quattro punti dell'ordine proposto). Una
correzione emersa scrivendo i test: vedi "Scoperta in corso d'opera".

## Motivazione

`vitest` esiste da
`specs/refinement/2026-09-03-testo-grassetto-corsivo.DONE.md` (29
test), ma copre solo `src/editor/inlineMarks.ts` +
serializer/deserializer. Tutto il resto del frontend — navigazione fra
journal/pagine, gestione delle scorciatoie globali, selezione nella
command palette, le altre estensioni Tiptap (`taskActions.ts`,
`moveBlock.ts`, `linkAutocomplete.ts`, `tagAutocomplete.ts`) — resta
verificato solo a mano, dentro `npm run tauri dev`. Utile finché il
lavoro procede in una sessione seguita passo passo con verifica
immediata; fragile per chiunque tocchi questo codice più avanti senza
rieseguire manualmente ogni percorso.

## Non tutto merita lo stesso trattamento

Non ogni file di `src/` deve avere test — molta UI (`SettingsPanel`,
`AboutPanel`, i pannelli React in generale) è già esplicitamente
esente per scelta ripetuta in tutte le spec di questa fase di
refinement ("coerente con l'assenza di un runner JS per **componenti**
nel progetto" — un test di rendering React richiederebbe
`@testing-library/react` + `jsdom`, dipendenze nuove, decisione a sé).
Questa spec riguarda solo **logica pura**, la stessa categoria di cosa
già testato in `inlineMarks.ts`: funzioni senza DOM, senza React,
facilmente isolabili.

## Ordine proposto (dal più al meno rischioso)

1. **`lib/shortcut.ts`**: `normalizeShortcut`, `matchesShortcut`,
   `formatShortcut`, `getShortcut` — logica di matching tastiera pura,
   zero DOM, il tipo di codice dove un edge case sbagliato (es. Shift
   riconosciuto quando non dovrebbe) rompe silenziosamente una
   scorciatoia senza errori visibili.
2. **`lib/journal.ts`**: `formatIsoDate`, `formatJournalHeader`,
   `parseTypedDate`, `journalDateFromPath` — `parseTypedDate` in
   particolare era già stata verificata con uno script Node usa-e-getta
   in `specs/refinement/2026-09-03-jump-to-date-command.DONE.md`:
   candidata naturale per diventare un test vero invece di uno script
   temporaneo perso a fine sessione.
3. **`editor/taskActions.ts`** (`cycleTaskState`): logica del ciclo a
   tre stati dei task, pura funzione su un documento ProseMirror in
   ingresso — stesso principio di `inlineMarks.ts`.
4. **`lib/paletteActions.ts`** (`buildActions`): già puro (context in,
   array di azioni out), test economico da scrivere.

`editor/linkAutocomplete.ts`/`tagAutocomplete.ts`/`moveBlock.ts` non in
lista: più intrecciati con lo stato vivo di ProseMirror (plugin con
side-effect sulla view), costerebbero un mock pesante per un beneficio
minore — se un giorno servisse, spec a parte.

## Scoperta in corso d'opera

**`cycleTaskState` non era la funzione pura descritta sopra**: legge
`editor.state`/`editor.view.dispatch` per davvero, stesso limite già
scritto per linkAutocomplete/tagAutocomplete/moveBlock — non emerso
finché non si è letto il file per scrivere il test. Estratta
`nextTaskState(text): { nextText, cursorDelta }`, la logica di
transizione a tre stati senza toccare `Editor`/`view` — refactor
comportamento-preservante (stesso identico calcolo, solo separato),
`cycleTaskState` la chiama invece di ripetere l'if/else inline. Il
resto della lista (shortcut/journal/paletteActions) era già puro come
previsto, nessun'altra sorpresa.

**`lib/shortcut.ts` ha un rischio non anticipato**: `IS_MAC` legge
`navigator.platform` una volta sola al caricamento del modulo — Node
22 espone un `navigator` globale che riflette il sistema operativo
REALE della macchina (diverso fra questa macchina, macOS, e il runner
Linux della CI). Un test ingenuo sarebbe passato in locale e fallito
(o viceversa) in CI. Ogni test di `shortcut.test.ts` sceglie la
piattaforma esplicitamente con `vi.stubGlobal("navigator", ...)` +
`vi.resetModules()` + un import dinamico dopo lo stub, invece di
affidarsi a quella reale.

## Fuori scope

- Test di integrazione end-to-end (avviare l'app vera, simulare click):
  richiederebbe Playwright/WebDriver + il setup per pilotare una
  finestra Tauri, un salto di complessità enorme rispetto a test di
  funzioni pure — non proposto qui.
- Test dei componenti React (`SettingsPanel`, `CommandPalette` come
  componente renderizzato): stessa scelta già presa ripetutamente in
  questa fase di refinement, non riaperta.

## Verifica

`npm run test`: 77 test totali (48 nuovi: 17 in `shortcut.test.ts`, 16
in `journal.test.ts`, 10 in `taskActions.test.ts`, 5 in
`paletteActions.test.ts`), tutti verdi. `npm run typecheck`, `cargo
test`, `cargo clippy --all-targets -D warnings`, `cargo fmt --check`
— puliti (il refactor di `nextTaskState` è l'unico cambio non-test,
comportamento-preservante). Nessuna verifica manuale necessaria, è la
natura stessa di questa spec.
