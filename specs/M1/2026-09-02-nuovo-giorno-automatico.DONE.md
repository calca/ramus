# Nuovo giorno aggiunto automaticamente a mezzanotte

Stato: implementata. Corregge una lacuna in M1 (già "completa" — la
milestone non riapre, questa spec la tappa senza rimettere in
discussione il resto).

## Motivazione

Verificato nel codice: `open_today()` viene chiamato solo una volta
(al mount di `App.tsx`, e di nuovo dopo un cambio vault) — nessun
timer, nessun controllo periodico. Se l'app resta aperta a cavallo di
mezzanotte, la lista dei giorni resta ancorata a quello di quando è
stata aperta: non appare il nuovo giorno, e il bottone "Oggi" scrolla
comunque al vecchio `pages[0]`, che non è più oggi.

## Rilevamento del cambio giorno, interamente lato frontend

Nessuna modifica a `ramus-core`/ai command Tauri: `open_today()` è già
idempotente e calcola sempre la data corrente al momento della
chiamata (`JournalDate::today()`, `chrono::Local`) — il problema è
solo che `App.tsx` non lo richiama mai una seconda volta. Il confronto
"che giorno è oggi lato JS" contro "che giorno è il primo della
lista" usa `formatIsoDate(new Date())`/`journalDateFromPath`, già
esistenti in `src/lib/journal.ts` — nessuna nuova utility di date.

```ts
function needsNewDay(): boolean {
  const first = pagesRef.current[0];
  return !first || journalDateFromPath(first.path) !== formatIsoDate(new Date());
}
```

### Quando si controlla

Due trigger, non uno solo — coprono casi diversi:

1. **Al ritorno di focus/visibilità della finestra**
   (`document.addEventListener("visibilitychange", ...)` o
   `window.addEventListener("focus", ...)`): copre il caso comune,
   l'utente chiude il laptop la sera e lo riapre il giorno dopo — il
   controllo scatta esattamente quando torna a guardare l'app, non
   prima né dopo.
2. **Controllo periodico leggero** (`setInterval`, 60s — stesso ordine
   di grandezza già proposto per il tick di M3): copre il caso raro in
   cui la finestra resta a fuoco ininterrottamente attraverso
   mezzanotte (es. schermo sempre acceso). Il confronto è una stringa,
   costo trascurabile ogni 60s.
3. **Click su "Oggi"**: `scrollToToday` esegue prima lo stesso
   controllo (vedi sotto) — un modo manuale e immediato di ottenere il
   giorno vero senza aspettare gli altri due trigger.

### Cosa succede quando serve un nuovo giorno

```ts
async function ensureToday() {
  if (!needsNewDay()) return;
  const today = await openToday(); // idempotente, esiste già
  const next = [today, ...pagesRef.current];
  pagesRef.current = next;
  setPages(next);
}
```

- **Nessuno scroll automatico**: il nuovo giorno appare in cima alla
  lista senza spostare la vista di chi sta leggendo/scrivendo altrove
  — coerente con non interrompere mai un flusso di scrittura in corso
  (stesso principio già seguito per gli avvisi di modifica esterna,
  mai invasivi). Se l'utente è già scrollato in cima lo vede comparire
  naturalmente.
- **Nessun impatto sulle sezioni esistenti**: il vecchio "oggi" (ora
  ieri) non viene toccato, ricaricato, né perde eventuali modifiche
  non salvate — cambia solo la sua posizione nell'elenco (da indice 0
  a indice 1) e di conseguenza smette di avere `isToday` (già derivato
  da `index === 0` in `pages.map`, nessuna modifica a
  `JournalSection` necessaria: la classe "oggi" si sposta da sola col
  nuovo ordine).
- **Nessuna duplicazione**: dopo l'inserimento `pages[0]` è già il
  giorno corretto, quindi `needsNewDay()` torna `false` ai controlli
  successivi finché la data non cambia di nuovo — nessun bisogno di un
  flag separato "già controllato oggi".
- **Nessun impatto sullo scroll infinito**: si inserisce solo in cima,
  il cursore di paginazione (`before` in `fetchNextBatch`, basato
  sull'ultimo giorno già caricato in fondo) resta invariato.

### `scrollToToday` diventa `ensureToday` + scroll

```ts
const scrollToToday = useCallback(async () => {
  await ensureToday();
  const today = pagesRef.current[0];
  if (today) {
    scrollToPath(today.path);
  }
}, [scrollToPath]);
```

Cambia da sincrono ad async (un `await` in più prima dello scroll) —
impercettibile nel caso comune (`needsNewDay()` è quasi sempre
`false`, l'unica chiamata extra è il confronto di stringhe), un
piccolo ritardo di rete/disco solo nel caso raro in cui serva
davvero creare il nuovo giorno.

## Fuori scope per questa spec

- Notifica/banner "è arrivato un nuovo giorno": il nuovo giorno appare
  silenziosamente in cima, coerente con "zero attrito" — nessun
  annuncio esplicito necessario.
- Comportamento diverso se l'app è rimasta aperta più di un giorno
  intero (es. il computer è rimasto sospeso per una settimana): la
  stessa logica gestisce comunque il caso, `open_today()` restituisce
  sempre il giorno corrente reale, i giorni saltati nel mezzo restano
  semplicemente assenti dalla lista come qualunque altro giorno senza
  file — comportamento già esistente (`list_journals` "non genera
  placeholder"), non una novità introdotta qui.

## Domande aperte

Nessuna bloccante — la correzione ripristina il comportamento atteso
("apertura automatica del journal di oggi", bullet di M1 in SPEC.md)
più che introdurne uno nuovo da negoziare.

## Test da scrivere

Nessuno lato core (`open_today` è già testato, invariato). Nessun
test frontend nuovo, coerente con l'assenza di un runner JS per
componenti nel progetto.

## Verifica

`npm run typecheck` per la parte automatizzabile. Il rollover reale
richiede aspettare la mezzanotte o forzare la data di sistema — non
verificabile in questo sandbox: serve un giro manuale in
`npm run tauri dev` (es. cambiando temporaneamente l'ora di sistema).
