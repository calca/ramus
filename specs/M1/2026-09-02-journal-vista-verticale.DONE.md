# Vista Journal verticale (stile Logseq)

Stato: implementata. `BATCH_SIZE = 14` giorni per blocco; salto a data
arbitraria incluso (vedi sotto).

## Motivazione

La vista Journal attuale (M1) mostra un giorno alla volta, con
precedente/successivo/oggi come comandi discreti. Si passa a una vista
verticale che aggrega i journal in un unico scroll continuo, come la vista
"Journals" di Logseq: oggi in cima, scendendo si va indietro nel tempo,
ogni giorno è la sua sezione con un header data, editabile sul posto.

Questa spec sostituisce la parte di navigazione fra giorni descritta in
SPEC.md (M1: "Navigazione fra giorni") con l'aggregazione verticale.
Riguarda solo la vista Journal: l'apertura di una pagina singola in
`pages/` (quando esisterà, con M2/i link) resta un caso separato, non
toccato da questa spec.

## Comportamento

- All'avvio: il journal di oggi è la prima sezione in cima, sempre
  presente (creato vuoto se non esiste, come oggi con `open_today`).
- Sotto, in ordine decrescente di data, i journal dei giorni precedenti
  **che esistono già come file**. Un giorno senza file non genera una
  sezione vuota: viene semplicemente saltato, si passa al giorno
  precedente che ha davvero contenuto.
- Caricamento a pagine (infinite scroll): si carica un primo blocco di
  `BATCH_SIZE = 14` giorni; quando lo scroll si avvicina al fondo della
  lista caricata, si richiede il blocco successivo (i giorni
  immediatamente precedenti all'ultimo caricato) e si accoda.
- Ogni sezione è un editor a blocchi indipendente (stesso outliner di
  M1): scrittura, indent/outdent, debounce di salvataggio 500ms e flush
  su blur/chiusura valgono per-sezione, non per l'intera vista.
- Bottone "Oggi" in header: scrolla la vista fino in cima (il giorno di
  oggi è sempre la prima sezione caricata, non richiede una fetch).
  Sostituisce i pulsanti precedente/successivo di M1, che perdono senso
  in una vista aggregata.
- Selettore data in header (vedi "Salto a data"): permette di andare
  direttamente a un giorno lontano senza scorrere manualmente.

## Core (`ramus-core`)

Nuova funzione in `vault.rs`, accanto a `open_today`:

```rust
/// Elenca i journal esistenti in ordine decrescente di data (più recente
/// prima), strettamente precedenti a `before` (se `None`, si parte dal
/// giorno più recente esistente). Non genera placeholder: un giorno senza
/// file non compare. `limit` è il numero massimo di pagine restituite.
pub fn list_journals(
    &self,
    before: Option<JournalDate>,
    limit: usize,
) -> Result<Vec<Page>, CoreError>
```

Implementazione: legge le voci di `journals/`, tiene solo i nomi file che
combaciano con `YYYY-MM-DD.md`, li converte in `JournalDate`, scarta quelli
`>= before` (se specificato), ordina decrescente, prende i primi `limit` e
per ciascuno chiama `read_page` (riuso diretto, nessuna duplicazione del
parsing).

Non serve includere "oggi" in questa funzione: resta responsabilità di
`open_today`, che già garantisce esistenza e creazione. Il frontend
compone le due chiamate (vedi sotto).

## Command Tauri

```
list_journals(before: Option<String>, limit: u32) -> Result<Vec<Page>, CoreError>
```

Wrapper sottile 1:1 sul core: converte `before` (stringa ISO 8601 o
assente) in `JournalDate`, chiama `Vault::list_journals`, propaga
l'errore. `limit` va validato/clampato a un massimo ragionevole lato core
(es. 90) per evitare richieste degeneri dal frontend — dettaglio di
implementazione, non parte del contratto pubblico.

## Frontend

- Stato: `pages: Page[]` (non più un singolo `page`), ordinato dal più
  recente al più vecchio. Il primo elemento è sempre oggi.
- Flusso iniziale: `openToday()` → primo elemento; poi
  `listJournals({ before: pages[0].path date, limit: BATCH_SIZE })` per
  il primo blocco di giorni precedenti.
- Scroll infinito: quando l'ultima sezione caricata entra in viewport
  (`IntersectionObserver` su un elemento sentinella in fondo alla lista),
  richiedere `listJournals({ before: ultimoGiornoCaricato, limit: BATCH_SIZE })`
  e accodare il risultato. Se il risultato è vuoto, non ci sono più
  giorni: smettere di osservare (fine della cronologia).
- Un `<Editor>` per sezione, key `page.path` come oggi — nessuna modifica
  al componente Editor stesso.
- Dirty tracking: da booleano singolo a mappa `Record<path, boolean>` (una
  voce per sezione aperta).
- File watcher (`vault://file-changed`): il match "è la pagina che ho
  aperto?" diventa una ricerca nell'array `pages` per `path`; il resto
  della logica (ricarica se non dirty, banner di avviso se dirty) non
  cambia, solo si applica alla sezione specifica invece che alla pagina
  unica.
- Flush su blur/chiusura finestra: va esteso a *tutte* le sezioni montate
  (un ref per editor, o un registry di ref), non solo a una.
- Header di sezione: data in formato leggibile (es. giorno della
  settimana + ISO), amber solo sulla sezione di oggi (coerente con la
  regola di palette "amber solo giorno corrente e blocco in focus").

## Salto a data (calendario)

Non è una lista "riavviata" ancorata alla data scelta: la vista resta
sempre ancorata a oggi (nessun reset), il salto scorre semplicemente
fino al giorno scelto, caricando quanto manca nel mezzo. Evita di perdere
lo scroll già fatto e tiene un solo modello di caricamento (sempre in
avanti da oggi verso il passato).

- Controllo: `<input type="date">` nativo in header, `max` impostato a
  oggi (non si può saltare nel futuro). Nessuna libreria di calendario
  nuova: l'input nativo basta e rispetta la regola "nessuna dipendenza
  nuova senza motivo".
- Alla scelta di una data:
  1. Se una sezione con data `<=` quella scelta è già fra le `pages`
     caricate (ricordando che i giorni senza file sono saltati: si cerca
     la prima con data `<=`, non un match esatto), si scrolla lì
     (`scrollIntoView({ behavior: "smooth" })`), fine.
  2. Altrimenti si continua a chiamare `list_journals(before: ultimoCaricato,
     limit: BATCH_SIZE)` in sequenza, accodando ogni blocco, finché non si
     ottiene una pagina con data `<=` quella scelta oppure un blocco
     restituisce meno di `BATCH_SIZE` risultati (fine della cronologia:
     non esistono giorni precedenti a quello richiesto). Poi si scrolla
     alla pagina più vicina trovata.
- Nessuna nuova funzione core o command: riusa `list_journals` così com'è,
  solo in loop lato frontend. Il contratto backend non cambia.

## Fuori scope per questa spec

- Apertura di pagine `pages/*.md` (arriverà con i link in M2, vista
  separata, non aggregata).
- Virtualizzazione della lista (smontare gli editor delle sezioni molto
  scrollate fuori viewport per limitare le istanze Tiptap attive): non
  necessaria all'inizio, ma è un limite noto se la cronologia diventa
  molto lunga (anni di journal). Da riconsiderare se diventa un problema
  di performance reale, non preventivamente.

## Test da scrivere (core)

- `list_journals` rispetta l'ordine decrescente.
- `list_journals` salta le date senza file corrispondente (nessun
  placeholder).
- `list_journals` rispetta `before` (esclusivo) e `limit`.
- `list_journals` ignora file in `journals/` che non combaciano col
  pattern `YYYY-MM-DD.md` (es. file spuri, non validi come data).
- `list_journals` su vault vuoto restituisce lista vuota, non errore.
