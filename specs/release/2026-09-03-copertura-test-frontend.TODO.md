# Estendere la copertura di test del frontend

Stato: proposta, da implementare. Nessuna domanda bloccante — solo
una priorità da confermare (vedi "Ordine proposto").

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

## Fuori scope

- Test di integrazione end-to-end (avviare l'app vera, simulare click):
  richiederebbe Playwright/WebDriver + il setup per pilotare una
  finestra Tauri, un salto di complessità enorme rispetto a test di
  funzioni pure — non proposto qui.
- Test dei componenti React (`SettingsPanel`, `CommandPalette` come
  componente renderizzato): stessa scelta già presa ripetutamente in
  questa fase di refinement, non riaperta.

## Verifica

`npm run test` con i nuovi file (`shortcut.test.ts`,
`journal.test.ts`, `taskActions.test.ts`, `paletteActions.test.ts`)
verdi, `npm run typecheck` pulito — nessuna verifica manuale
necessaria, è la natura stessa di questa spec.
