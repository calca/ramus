# Task nei blocchi (`[ ]` / `[x]`)

Stato: implementata. Riconoscimento/decorazione, click sul marker e
ciclo a tastiera (`Mod-Enter`) implementati esattamente come proposto
(domande 1 e 2 confermate). La sezione "Spostare un task a oggi" è
stata **ridisegnata** durante la conferma: non più un bottone manuale
per singolo task, ma uno spostamento automatico al cambio di giorno,
su una finestra configurabile — vedi sotto, sezione riscritta per
intero.

## Motivazione

Sesto pezzo di M4: marcare un blocco come attività da fare/fatta,
diretto nel testo del blocco — non un tipo di blocco separato, un
prefisso riconosciuto dentro `content` (esattamente come `[[link]]` e
`#tag`, M2: testo letterale, decorato a video, mai una struttura dati
nuova).

## Formato: quello di Obsidian/GitHub, non uno inventato

SPEC.md principio 4: "il formato su disco è compatibile con
Obsidian". La sintassi task di Obsidian (e GitHub-flavored markdown)
è già `- [ ] testo` (da fare) / `- [x] testo` (fatto) — un blocco
Ramus è già `- ` + `content`, quindi un task è semplicemente un
blocco il cui `content` **inizia** con `[ ] ` o `[x]`/`[X] `:

```
- [ ] Comprare il latte
- [x] Chiamare il commercialista
  - [ ] Preparare i documenti prima
```

**Zero modifiche al modello dati o al parser**: `Block.content` resta
`String`, `parser.rs`/`serializer.ts`/`deserializer.ts` non cambiano —
non c'è nulla da riconoscere a livello di parsing dei blocchi, solo di
**decorazione** nell'editor, stesso principio già scritto per link/tag
in `specs/M2/2026-09-02-link-tag-parsing.DONE.md` ("niente mark o nodi
ProseMirror... la sintassi vive solo come decorazione visiva").
Round-trip del parser (`parse(render(page)) == page`) non tocca
minimamente questa spec: il testo del blocco è testo, punto.

## Riconoscimento e decorazione

Nuovo `src/editor/taskHighlight.ts`, stesso schema di
`linkTagHighlight.ts` (`Extension` + `addProseMirrorPlugins` +
`props.decorations`, pattern regex su `node.text`):

```ts
const TASK_PATTERN = /^\[( |x|X)\] /;
```

Per ogni blocco il cui testo combacia da posizione 0:

- Il marker (`[ ] `/`[x] `) prende la classe `.editor-task-marker` —
  **resta testo visibile**, non un widget/checkbox HTML iniettato:
  stessa scelta già fatta per `[[link]]` ("le parentesi quadre
  restano visibili, nessun collasso stile live preview", principio
  esplicito già scritto in M2, qui riapplicato) — coerenza con quanto
  già deciso, non una nuova decisione.
- Se è `[x]`/`[X]` (fatto), l'intero `<li>` del blocco prende una
  classe aggiuntiva `.task-done` (stesso meccanismo già in uso per
  `.block-focused` su focus — una classe sull'elemento lista, non una
  decorazione di ogni singolo carattere).

## Stile

```css
.editor-task-marker {
  cursor: pointer;
  font-weight: 600;
}

.ramus-editor li.task-done > p {
  text-decoration: line-through;
  opacity: 0.55;
}
```

Il marker non usa `--ramus-sap`/`--ramus-amber` (riservati a
bullet/guide/giorno corrente e a link/tag/notifiche — vedi
CLAUDE.md/SPEC.md, "l'amber non va usato per link, tag o notifiche" e
il sap è già il colore "strutturale"): resta `--ramus-ink` (colore di
default del testo), solo più marcato (`font-weight`) e cliccabile. Il
segnale visivo di "fatto" è lo strikethrough + opacità ridotta sul
resto del blocco, non un colore nuovo da aggiungere alla palette.

## Interazione

### Click sul marker: toggle `[ ]` ⇄ `[x]`

Stesso pattern già in `Editor.tsx` per `.editor-link` (click handler
su `EditorContent`, `event.target.closest(...)`): click su
`.editor-task-marker` sostituisce il singolo carattere fra parentesi
(spazio ↔ `x`) con una transazione ProseMirror mirata — non riscrive
tutto il blocco, solo quel carattere. Nessun click possibile su un
blocco che non è già un task (niente checkbox "fantasma" su blocchi
normali).

### Scorciatoia da tastiera: ciclo su tre stati

Scorciatoia **fissa**, non nel registro configurabile di
`specs/M4/2026-09-02-scorciatoie-configurabili.DONE.md` — stessa
motivazione già scritta per il riordino blocchi
(`specs/M4/2026-09-02-riordino-blocchi-tastiera.DONE.md`): vive nella
keymap ProseMirror dell'editor, non in un listener a livello finestra.
Default proposto: `Mod-Enter`.

Sul blocco a fuoco, in ciclo a ogni pressione:

```
blocco normale → "[ ] " + contenuto esistente (diventa un task da fare)
       ↑                              ↓
"[x] " + contenuto  ←——————————  toggle a "fatto"
       ↓
blocco normale (rimuove il marker, torna testo semplice)
```

Tre stati, una sola scorciatoia, si preme più volte per scorrerli —
stesso principio "un tasto solo" già usato altrove nell'app (es.
compact-toggle). Il cursore resta nella stessa posizione relativa al
testo (non salta all'inizio/fine) quando possibile.

## Spostare un task a oggi (ridisegnata: automatico, non un bottone)

**Aggiornamento**: il testo originale proponeva un bottone icona `→`
per singolo task, a comparsa su hover. In fase di conferma l'utente ha
chiesto un meccanismo diverso: **nessun bottone**, spostamento
**automatico** al cambio di giorno, per una ragione concreta di
utilizzo — un'app chiusa tutta la settimana (lun-ven di lavoro) e
riaperta il lunedì successivo lascia diversi giorni di task non fatti
accumulati, che un bottone manuale per-task non risolve comodamente
tutti insieme.

Un task `[ ] ` non fatto scritto giorni fa tende a scomparire
scorrendo verso il basso nella vista journal. Al cambio di giorno,
Ramus scansiona una finestra di giorni passati e sposta (non copia,
non un riferimento — un vero spostamento, coerente con "niente block
reference" già fuori scope in SPEC.md) ogni task ancora `[ ] ` in
fondo al journal di oggi.

### Ambito: solo journal, non pagine

Solo i giorni di journal passati vengono scansionati, non le pagine
(`pages/*.md`) — un task scritto dentro una pagina non "scompare"
scorrendo nel tempo come un giorno di journal, la stessa urgenza non
si applica.

### Solo task non fatti

Solo `[ ] ` (non fatti) vengono spostati — un task `[x]`/`[X]` già
fatto non ha bisogno di essere "riportato sotto gli occhi".

### Finestra di scansione, configurabile

`Config.task_rollover_days: u32` (default 7): quanti giorni indietro
da oggi (escluso oggi stesso) vengono scansionati per i task rimasti
aperti — abbastanza per coprire un weekend o una settimana intera di
app chiusa (lun-ven più il weekend, il caso che ha motivato il
redesign), senza dover riscansionare l'intero journal ad ogni cambio
di giorno. `Config.task_rollover_enabled: bool` (default `true`):
interruttore in Impostazioni per disattivare del tutto il
comportamento.

### Trigger: cambio di giorno, non ogni avvio

Al momento in cui l'app apre/riapre il journal di "oggi" per la prima
volta in questa sessione — sia all'avvio (caso comune: app chiusa e
riaperta un altro giorno), sia al rollover di mezzanotte a app già
aperta (`ensureToday()`, M1) — **prima** di leggere/aprire la pagina
di oggi. Non un timer separato: si aggancia esattamente ai due punti
in cui `App.tsx` sta già per chiamare `openToday()`.

### Cosa si sposta

L'intero sottoalbero (il task e i suoi figli, se ne ha) — stesso
principio già scritto per il riordino da tastiera
(`specs/M4/2026-09-02-riordino-blocchi-tastiera.DONE.md`, "il blocco
spostato porta con sé i suoi figli"): un sotto-task o una nota
attaccata al task non deve restare orfana nel giorno vecchio. La
scansione ricorsiva: un task annidato sotto un blocco che *non* è
esso stesso un task viene comunque trovato ed estratto (il blocco
genitore non-task resta al suo posto, solo il sottoalbero del task
si sposta).

### Dove finisce in "oggi"

In fondo ai blocchi di primo livello del journal di oggi (append),
in ordine dal giorno scansionato più vecchio al più recente.

### Meccanica (ordine che evita di perdere dati)

Nuovo modulo `crates/ramus-core/src/rollover.rs`,
`roll_over_unfinished_tasks(vault, days_back) -> RolloverOutcome`:

1. Scansiona (solo lettura) i `days_back` giorni di journal
   precedenti a oggi che esistono, estraendo ricorsivamente ogni
   sottoalbero-task non fatto e collezionandoli tutti in memoria.
   Nessuna scrittura ancora.
2. Se non c'è nulla da spostare: nessuna scrittura, no-op completo
   (caso comune, costo trascurabile).
3. Legge/crea la pagina di oggi (`open_today`), aggiunge tutti i task
   raccolti in fondo, **scrive prima questa** (`write_page`). Se
   fallisce, nessuna sorgente è stata toccata — tutti i task restano
   dov'erano, nessuna perdita.
4. Solo dopo il salvataggio della destinazione, riscrive ogni pagina
   sorgente toccata senza i task spostati (`write_page` per ciascuna).
   Scrivere-poi-cancellare, mai il contrario — un fallimento a metà fra
   più sorgenti duplica nel peggiore dei casi (un task resta sia in
   "oggi" sia nella vecchia sorgente non ancora riscritta), non perde
   mai dati.

Il command Tauri `roll_over_unfinished_tasks` (nessun argomento, legge
`task_rollover_enabled`/`task_rollover_days` da `Config`) **non**
segna nessuna delle pagine toccate come self-write
(`mark_self_write`): le sezioni già montate nella vista journal per i
giorni sorgente devono restare rilevabili dal file watcher come
modifica "esterna", per riusare esattamente il percorso già gestito in
`App.tsx` (`vault://file-changed` → ricarica silenziosa se non dirty,
avviso se dirty) — nessuna logica nuova lato frontend per questo
caso, l'operazione è trattata come se un altro processo avesse
toccato i file, cosa che di fatto avviene.

### Impostazioni

Nuova sezione "Task": un interruttore ("Sposta automaticamente a oggi
i task non fatti rimasti indietro", attivo di default) e, solo se
attivo, un selettore per l'ampiezza della finestra (3/7/14/30 giorni,
stesso trattamento a scelta fissa già usato per l'intervallo di sync
Git — non un campo libero).

### Solo tastiera/automatico in questa spec

Nessun bottone manuale, nessuna scorciatoia da tastiera per uno
spostamento singolo: l'intero meccanismo è automatico e silenzioso
(nessun banner/notifica quando sposta qualcosa — coerente con la
sobrietà generale, vedi "Fuori scope").

## Fuori scope per questa spec

- Spostare anche task `[x]` già fatti: solo `[ ] ` non fatti,
  confermato.
- Scansionare anche le pagine (`pages/*.md`), non solo il journal:
  confermato, solo giorni di journal.
- Notifica/banner quando lo spostamento automatico avviene: silenzioso,
  coerente con la sobrietà generale — l'utente lo scopre trovando i
  task in cima a oggi, non da un avviso separato.
- Bottone "riprova ora"/spostamento manuale extra oltre a quello
  automatico: non richiesto, l'interruttore in Impostazioni è
  sufficiente per chi non lo vuole.
- Vista/pannello "tutti i task aperti nel vault" (un'agenda): servirebbe
  scansionare tutto il vault, esattamente il lavoro già fatto
  dall'indice SQLite (M2) — estensione naturale ma separata, non
  richiesta qui. Se dovesse servire, `extract_tags`-style extraction
  nell'indice (`crates/ramus-core/src/index.rs`) è il punto naturale
  da estendere, non toccato in questa spec.
- Stati intermedi tipo "in corso"/`DOING` (Logseq ne ha diversi): solo
  due stati (da fare/fatto), coerente con la sintassi Obsidian/GFM
  standard a due stati.
- Data di scadenza, priorità, o altri metadati sul task: solo lo stato
  fatto/da fare.
- Riconoscimento di sintassi task in mezzo al testo (non all'inizio
  del blocco): solo prefisso, stesso principio di `[[link]]` che
  richiede delimitatori precisi, non un match libero ovunque nel
  blocco.

## Domande aperte

Nessuna: tutte e quattro risolte — 1 e 2 confermate come proposto
(`Mod-Enter`, ciclo a un tasto solo); 3 e 4 superate dal redesign
dell'intera sezione "Spostare un task a oggi" (automatico, non più un
bottone per singolo task — vedi sopra), coi due nuovi dettagli emersi
in conferma (finestra configurabile, solo journal) già incorporati.

## Test da scrivere

**Editor** (riconoscimento, click, ciclo): nessuno lato core, nessun
test frontend nuovo — stessa scelta già fatta per gli altri
autocomplete/estensioni editor (assenza di un runner JS per
componenti nel progetto). La matematica di posizione di
`cycleTaskState` (la parte più a rischio di un bug fuori-per-uno,
stesso tipo di rischio già affrontato per il riordino blocchi) è stata
verificata con uno script Node usa-e-getta, non committato: ciclo
completo normale→`[ ] `→`[x] `→normale con tracciamento del cursore,
più il caso limite del blocco vuoto.

**Rollover automatico** (`crates/ramus-core/src/rollover.rs`): lato
core, con test veri (a differenza del resto della spec) perché tocca
filesystem/dati reali, non solo editor/CSS:

- Sposta un task `[ ] ` di ieri a oggi, lo rimuove dalla sorgente.
- Lascia sul posto task `[x]` già fatti e blocchi normali.
- Il sottoalbero (figli inclusi) segue il task spostato.
- Trova un task annidato sotto un blocco che non è esso stesso un task.
- Ignora giorni fuori dalla finestra di scansione configurata.
- No-op completo (zero scritture, nessun file "oggi" creato) su un
  vault senza task aperti nella finestra.
- Più giorni con task aperti: raccolti nell'ordine dal più vecchio al
  più recente.

Più un test su `Config` per i nuovi default
(`task_rollover_enabled`/`task_rollover_days`).

## Verifica

`cargo test` (114 test, +8 da questa spec), `cargo clippy`,
`cargo fmt --check` e `npm run typecheck` puliti. Non verificabile in
questo sandbox: il giro completo in `npm run tauri dev` — in
particolare che un file scritto da Ramus con task resti apribile e
visivamente corretto in un editor Obsidian reale (coerenza con
SPEC.md principio 4), che un file `.md` con task scritto da Obsidian
venga letto e decorato correttamente da Ramus, e il comportamento
end-to-end dello spostamento automatico (chiudere l'app, modificare
manualmente la data di un file di test per simulare "giorni fa", far
ripartire l'app, verificare che il task compaia a oggi).
