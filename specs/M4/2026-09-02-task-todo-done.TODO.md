# Task nei blocchi (`[ ]` / `[x]`)

Stato: proposta, in attesa di conferma.

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
`specs/M4/2026-09-02-scorciatoie-configurabili.TODO.md` — stessa
motivazione già scritta per il riordino blocchi
(`specs/M4/2026-09-02-riordino-blocchi-tastiera.TODO.md`): vive nella
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

## Fuori scope per questa spec

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

1. Scorciatoia di default `Mod-Enter` per il ciclo a tre stati — va
   bene, o preferisci qualcos'altro (es. `Mod-Shift-Enter`, per non
   rischiare ambiguità con un futuro uso di `Mod-Enter` altrove)?
2. Ciclo a tre stati con un solo tasto (proposto) — o preferisci due
   azioni distinte (una per "rendi/rimuovi task", una per "segna
   fatto/da fare"), più tasti ma più esplicite?

## Test da scrivere

Nessuno lato core: zero modifiche a `ramus-core` (il parser non
cambia, vedi sopra). Nessun test frontend nuovo, coerente con
l'assenza di un runner JS per componenti/estensioni editor nel
progetto (stessa scelta già fatta per gli altri autocomplete e per il
riordino blocchi).

## Verifica

Nessuna parte automatizzabile in questo sandbox: comportamento
puramente editor/CSS. Serve un giro manuale in `npm run tauri dev` —
verificare in particolare che un file scritto da Ramus con task resti
apribile e visivamente corretto in un editor Obsidian reale (coerenza
con SPEC.md principio 4), e viceversa che un file `.md` con task
scritto da Obsidian venga letto e decorato correttamente da Ramus.
