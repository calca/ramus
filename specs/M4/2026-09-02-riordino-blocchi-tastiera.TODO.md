# Riordino blocchi da tastiera (Alt+Su/Giù)

Stato: proposta, in attesa di conferma.

## Motivazione

Quarto pezzo di M4, parte dell'idea "keyboard focused". Oggi
l'outliner ha indent/outdent (Tab/Shift-Tab) e creazione/merge
(Invio/Backspace), ma nessun modo di riordinare due blocchi fratelli
senza tagliare/incollare a mano — un'operazione comune in un outliner
(Logseq/Workflowy la hanno entrambi su Alt+Su/Giù).

## Scorciatoia fissa, non configurabile

Come le altre scorciatoie dell'editor (Tab, Shift-Tab, Invio,
Backspace): vive nella keymap ProseMirror configurata alla creazione
dell'editor, non nel registro configurabile di
`specs/M4/2026-09-02-scorciatoie-configurabili.TODO.md` — stessa
motivazione già scritta lì, non ripetuta. `Alt-ArrowUp` /
`Alt-ArrowDown` (sintassi keymap Tiptap/ProseMirror, verificata contro
`prosemirror-keymap`: i modificatori si scrivono `Mod-`/`Alt-`/
`Shift-` come prefisso).

## Comportamento

- **Alt+Su**: il blocco a fuoco si scambia di posizione col fratello
  **precedente** allo stesso livello di indentazione. Se è già il
  primo fratello, nessun effetto (non risale di livello — quello resta
  Shift-Tab, un'azione diversa).
- **Alt+Giù**: simmetrico, scambio col fratello **successivo**. Ultimo
  fratello → nessun effetto.
- Il blocco spostato **porta con sé i suoi figli** (l'intero sottoalbero
  si sposta come unità, non solo la riga) — coerente con come
  indent/outdent trattano già i sottoalberi.
- Il focus e la posizione del cursore restano nel blocco spostato dopo
  lo scambio (l'utente può premere Alt+Giù più volte di seguito per
  scendere la lista, non deve ricliccare).
- Funziona solo fra fratelli **allo stesso genitore**: non sposta un
  blocco dentro/fuori un gruppo di figli diversi (quello resta
  Tab/Shift-Tab). Un blocco senza fratello nella direzione richiesta
  (primo/ultimo) semplicemente non fa nulla — nessun errore, nessun
  suono, coerente con la sobrietà del resto dell'editor.

## Implementazione

Nuova estensione in `src/editor/extensions.ts`, accanto a
`OutlinerBackspace` (stesso schema: `Extension.create` +
`addKeyboardShortcuts`):

```ts
const MoveBlock = Extension.create({
  name: "moveBlock",
  addKeyboardShortcuts() {
    return {
      "Alt-ArrowUp": ({ editor }) => moveListItem(editor, "up"),
      "Alt-ArrowDown": ({ editor }) => moveListItem(editor, "down"),
    };
  },
});
```

`moveListItem`: non esiste un comando pronto in
`prosemirror-schema-list` (verificato nel sorgente: espone solo
`sinkListItem`/`liftListItem`/`splitListItem`/`wrapInList`, nessun
"move") — serve una funzione scritta apposta. Algoritmo:

1. Dalla selezione corrente, risale al nodo `listItem` più vicino e al
   suo genitore (`bulletList`).
2. Trova l'indice del `listItem` fra i figli del genitore, e quello
   del fratello target (indice ± 1). Se non esiste (bordo della
   lista), no-op.
3. Costruisce una transazione ProseMirror che scambia le due porzioni
   di documento corrispondenti ai due `listItem` (range completo,
   sottoalbero incluso) — via `tr.delete` + `tr.insert` nell'ordine
   inverso, o l'equivalente con `Transform.replace`; i dettagli esatti
   dell'API si risolvono in fase di implementazione, non cambiano il
   comportamento osservabile descritto sopra.
4. Riposiziona la selezione dentro il blocco spostato (stessa
   posizione relativa al testo che aveva prima, quando possibile).

Nessuna modifica al formato su disco: il riordino è un'operazione
sull'albero di blocchi già rappresentabile (stesso `Block { content,
children }` di sempre), il salvataggio (debounce 500ms, invariato)
scrive semplicemente il nuovo ordine.

## Fuori scope per questa spec

- Riordino via drag & drop col mouse: solo tastiera in questa spec.
- Spostare un blocco a un livello di indentazione diverso nella stessa
  azione (quello resta Tab/Shift-Tab, azioni separate, come oggi).
- Spostare un blocco fra sezioni diverse del journal (giorni diversi)
  o fra una pagina e il journal: il riordino è sempre interno a una
  singola pagina/giorno, mai fra editor diversi.
- Rendere la scorciatoia configurabile (vedi sopra).

## Domande aperte

Nessuna bloccante — il comportamento (scambio con fratello, sottoalbero
incluso, focus che segue) sembra l'unica interpretazione ragionevole
data la convenzione già nota da altri outliner. Da confermare solo se
hai un'aspettativa diversa.

## Test da scrivere

Nessuno lato core: la logica vive interamente nell'editor
(ProseMirror/Tiptap), non in `ramus-core`. Nessun test frontend nuovo,
coerente con l'assenza di un runner JS per componenti/estensioni
editor nel progetto (stessa scelta già fatta per gli altri
autocomplete).

## Verifica

Nessuna parte automatizzabile in questo sandbox (comportamento
editor). Serve un giro manuale in `npm run tauri dev`: spostare un
blocco con figli su/giù, verificare che il sottoalbero segua, che il
cursore resti nel blocco spostato, e che il salvataggio (500ms dopo)
scriva il nuovo ordine sul file.
