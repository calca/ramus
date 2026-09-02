# Navigazione: apertura pagina dal click su `[[link]]`

Stato: implementata. Una semplificazione rispetto al testo originale:
la vista journal resta sempre montata (nascosta via CSS, come
descritto sotto — scroll ed editor preservati), ma `PageView` è
montata/smontata in JSX come `SettingsPanel`/`AboutPanel` invece di
restare anch'essa sempre viva. Costo accettato: rivisitare la stessa
pagina dopo essere tornati al journal la ricrea da zero (un piccolo
refetch + reinizializzazione dell'editor), non una perdita di dati
(il flush avviene comunque prima di ogni navigazione).

## Motivazione

Terzo pezzo di M2 in SPEC.md: "Pannello backlink sulla pagina aperta".
Questa spec copre la parte "pagina aperta" (apertura/navigazione); il
pannello backlink vero e proprio resta rimandato — serve l'indice
SQLite (secondo pezzo di M2, ancora da fare) per sapere chi linka a
cosa. Senza questa spec i `[[link]]` della spec precedente si possono
digitare e autocompletare ma non si possono aprire: è il pezzo che li
rende utili.

## Modello di vista

Oggi `App.tsx` ha un solo "modo": la vista journal verticale. Si
aggiunge un secondo modo, la vista di una singola pagina:

```ts
type View = { kind: "journal" } | { kind: "page"; page: Page };
```

- Lo stato del journal (`pages`, `hasMore`, posizione di scroll,
  istanze `Editor` già montate) **non viene distrutto** passando a
  `"page"`: resta montato nel DOM, solo nascosto via CSS (`display:
  none` sul contenitore, non un unmount condizionale in JSX) — al
  ritorno lo scroll è esattamente dov'era, nessuna ricarica. Stesso
  principio per il ritorno: la vista pagina resta montata ma nascosta
  quando si torna al journal, non viene ricreata da zero a ogni click
  sullo stesso link.
- Passare a `"page"`: flush di **tutti** gli editor montati (stesso
  `Promise.all` già usato in `onCloseRequested` — riusa
  `editorHandles`, non serve duplicarlo), poi `open_page(title)`
  (crea se manca — vedi spec precedente), poi
  `setView({ kind: "page", page })`.
- Tornare al journal: flush dell'editor della pagina aperta, poi
  `setView({ kind: "journal" })`.

## Click su un link

La decorazione `.editor-link` (dalla spec precedente) porta già il
testo del link nell'attributo `data-title` impostato al momento della
decorazione (nessun bisogno di ri-matchare la regex al click).

- `Editor.tsx` guadagna un prop opzionale `onLinkClick?: (title:
  string) => void`, cablato a un listener di click su
  `EditorContent` che intercetta `event.target.closest(".editor-link")`
  e legge `data-title`.
- `App.tsx` passa lo stesso `onLinkClick={navigateToPage}` a **ogni**
  istanza di `Editor` che monta — sia quelle dentro le sezioni journal
  sia quella (singola) della vista pagina: un link cliccato da
  qualunque punto dell'app porta allo stesso posto.
- `navigateToPage(title)`: chiama `open_page(title)` (crea la pagina se
  non esiste ancora — è il momento in cui un link "promesso" ma mai
  aperto si materializza come file, coerente con la spec precedente),
  poi passa a `view: { kind: "page", page }`.

## Vista pagina (`PageView.tsx`, nuovo componente)

- Header proprio dentro `<main>`: bottone "← Journal" (torna alla
  vista journal) + titolo della pagina (`page.title`, o lo slug come
  fallback se manca — coerente col fallback già usato altrove per
  pagine senza front-matter).
- Un solo `<Editor>` (lo stesso componente esistente, nessuna modifica
  oltre al nuovo prop `onLinkClick`) con `page` impostato alla pagina
  aperta — stesso debounce/flush/dirty-tracking già esistente, nessuna
  logica nuova di salvataggio.
- Nessun pannello backlink (fuori scope, vedi sotto): solo editor e
  bottone indietro.

## `app-header` durante la vista pagina

`JournalControls` (Oggi, salto a data) non ha senso mentre si guarda
una pagina — si nasconde (stesso meccanismo già usato per
`is-compact`: una condizione su `view.kind`, non su una nuova classe).
Il resto dell'header (logo, toggle compatta, ingranaggio impostazioni)
resta invariato, cross-view.

## File watcher esteso

La logica esistente in `App.tsx` (se il file cambia esternamente:
ricarica silenziosa se non dirty, avviso se dirty, mai sovrascrivere)
si estende con un ramo in più: se `view.kind === "page"` e il path
cambiato combacia `view.page.path`, si applica la stessa logica già
usata per le sezioni journal. Stesso pattern, non una logica nuova.

## Fuori scope

- Pannello backlink vero e proprio (chi linka a questa pagina): serve
  scansionare tutto il vault, è esattamente il lavoro dell'indice
  SQLite — spec a parte, dopo questa, prima del pannello.
- Rinominare o eliminare una pagina.
- Raggiungere una pagina da un posto diverso dal click su un
  `[[link]]` (es. un elenco di tutte le pagine, un risultato di
  ricerca): arriva con la ricerca full-text, ultimo pezzo di M2.
- Più pagine aperte contemporaneamente (tab, split view): una sola
  vista pagina alla volta, coerente con l'app a vista singola di oggi.
- Link ricorsivi visitabili all'infinito senza un modo di tornare
  indietro più di un passo (solo "← Journal", non una cronologia di
  navigazione tipo browser): se serve, è un'estensione piccola ma va
  decisa a parte.

## Verifica

Non testabile con `cargo test` per la parte di interazione (click,
apertura, ritorno, watcher su una pagina aperta) — `npm run typecheck`
copre la parte automatizzabile, il resto serve un giro manuale in
`npm run tauri dev`.
