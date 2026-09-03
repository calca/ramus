# Command palette: scorciatoie visibili, stato vuoto, match evidenziato

Stato: implementata. Decima spec della fase di refinement. Tre miglioramenti indipendenti alla stessa
`CommandPalette.tsx`, raggruppati in una spec sola perché toccano lo
stesso file e nessuno richiede l'altro.

## 1. Scorciatoia accanto alle azioni che ce l'hanno

**Payoff reale, dichiarato subito**: i cinque comandi della palette
(`paletteActions.ts`: today/return-journal, toggle-compact, settings,
about, cheatsheet) e le cinque scorciatoie configurabili
(`SHORTCUT_ACTIONS` in `lib/shortcut.ts`: command_palette, cheatsheet,
focus_mode, journal_prev_day, journal_next_day) si sovrappongono per
un solo id: `cheatsheet`. Oggi questo pezzo illumina **una sola riga
su cinque** ("Mostra scorciatoie" mostrerà la sua stessa scorciatoia,
`⌘/`). Non è inventato per sembrare più utile di quanto sia: il
meccanismo è generico e si accende da solo per qualunque azione
futura che guadagni un id condiviso con `SHORTCUT_ACTIONS` — oggi vale
per una voce, non per cinque, ed è comunque corretto farlo perché non
mostra mai un hint falso.

`getShortcut(shortcuts, actionId)` (già in `lib/shortcut.ts`) ritorna
già `""` per un id non registrato — nessuna logica nuova da scrivere
per il caso "nessuna scorciatoia", solo consumarla.

**Modifiche**:
- `CommandPaletteProps` guadagna `shortcuts: Record<string, string>`.
- `App.tsx`: `<CommandPalette shortcuts={config?.shortcuts ?? {}} ...>`
  (nessun guard `config &&` esiste già su questo pannello — usa
  optional chaining invece di aggiungerne uno, stesso principio già
  visto altrove nel file).
- Nel render di `item.kind === "action"`:
  ```tsx
  const shortcut = getShortcut(shortcuts, item.action.id);
  // ...
  <span className="palette-item-title">
    {item.action.label}
    {shortcut && <span className="palette-item-shortcut">{formatShortcut(shortcut)}</span>}
  </span>
  ```
- `.palette-item-title` passa da `display: block` a un flex-row
  (`justify-content: space-between`) così l'eventuale scorciatoia si
  allinea a destra — innocuo per tutti gli item senza secondo figlio
  (recent/create/date/hit), che restano visivamente identici.
- Nuova regola `.palette-item-shortcut`: monospace, `color:
  var(--ramus-stone)`, `font-weight: 400` (il titolo resta in grassetto,
  la scorciatoia no — stessa gerarchia di enfasi già in uso ovunque
  nell'app fra testo principale e metadato).

## 2. Stato vuoto invece di una lista che sparisce nel nulla

Oggi `{items.length > 0 && (<ul>...)}`: se una query non vuota non
produce nessun match (nessuna azione, nessun risultato di ricerca,
nessuna data valida, e una pagina con quel titolo esatto esiste già —
quindi niente nemmeno da "Crea") la palette non mostra assolutamente
nulla sotto il campo di testo. Sembra bloccata, non "zero risultati".

**Modifiche**: `const trimmedQuery = query.trim();` nel corpo del
componente (fuori dalla `useMemo`, riusata anche lì). Il ramo di
render diventa:
```tsx
{items.length > 0 ? (
  <ul className="palette-results">...</ul>
) : (
  trimmedQuery && <p className="palette-empty">Nessun risultato per «{trimmedQuery}»</p>
)}
```
A query vuota `items.length` non è mai zero (le azioni fisse ci sono
sempre), quindi il messaggio compare solo quando ha senso: query
digitata, zero risultati. Nuova regola `.palette-empty`: stesso
padding/margin di `.palette-results`, colore `--ramus-stone` (testo
secondario, stesso trattamento di `.palette-item-snippet`).

## 3. Match evidenziato in azioni e pagine recenti

Oggi solo i risultati di ricerca full-text (`item.hit.snippet_html`,
generato dal backend Rust) hanno il testo che ha fatto match in
grassetto. Azioni e pagine recenti sono filtrate per sottostringa
lato client (`action.label.toLowerCase().includes(lower)`) ma il
match non si vede — bisogna leggere tutta l'etichetta per capire
perché è comparsa.

**Modifiche**: nuova funzione pura in `CommandPalette.tsx` (solo
presentazione, non logica di dominio — non va in `lib/`):
```tsx
function highlightMatch(text: string, query: string): ReactNode {
  if (!query) return text;
  const index = text.toLowerCase().indexOf(query.toLowerCase());
  if (index === -1) return text;
  return (
    <>
      {text.slice(0, index)}
      <mark className="palette-match">{text.slice(index, index + query.length)}</mark>
      {text.slice(index + query.length)}
    </>
  );
}
```
Applicata a `item.action.label` (kind "action") e `item.title` (kind
"recent") con `trimmedQuery` come query — non a "create" (il testo
digitato è già interamente il titolo, evidenziarlo tutto non
aggiunge nulla) né a "hit" (ha già la propria evidenziazione dal
backend) né a "date" (l'etichetta è generata da `formatPrettyDate`,
non è un match testuale sulla query digitata).

Il titolo (`.palette-item-title`) è già `font-weight: 600; color:
var(--ramus-ink)` — il massimo dell'enfasi disponibile, in grassetto
non si vedrebbe un ulteriore "grassetto sopra il grassetto". Serve uno
sfondo, non un cambio di peso/colore: `<mark>` di default nel browser
è giallo, va sovrascritto (CLAUDE.md, "colori solo via variabili CSS,
mai hex inline"). **Sap, non amber**: la regola già in vigore
("l'amber non va mai usato per link, tag o notifiche", vedi
`.mock-head .today-dot` in `assets/palette.css`-adiacenti e i
commenti in `index.css`) riserva l'amber al giorno corrente/focus —
un'evidenziazione di ricerca non è nessuna delle due cose, ma resta
comunque fuori dal significato già assegnato all'amber; `--ramus-sap`
è il colore d'accento generico già usato per lo stato selezionato
della palette stessa (`.palette-item.is-selected`), coerente riusarlo
qui:
```css
.palette-match {
  background: color-mix(in srgb, var(--ramus-sap) 22%, transparent);
  border-radius: 3px;
  padding: 0 0.1em;
}
```

## Fuori scope

- Aggiungere scorciatoie configurabili nuove per le azioni che oggi
  non ne hanno (toggle-compact, settings, about, today): espanderebbe
  `SHORTCUT_ACTIONS`/la sezione Scorciatoie di Impostazioni, una
  richiesta diversa da "migliora la palette" — se servisse, spec a
  parte.
- Evidenziare il match anche nei risultati di ricerca full-text: già
  fatto lato backend (`snippet_html`), non toccato qui.
- Fuzzy matching (subsequence, non sottostringa esatta) per
  azioni/recenti: il filtro resta `includes()`, cambiarlo è
  un'estensione della logica di ricerca, non della sua presentazione.

## Domande aperte

Nessuna: tutte e tre le scelte di design (colore sap per il match,
posizione a destra della scorciatoia, messaggio di stato vuoto solo a
query non vuota) motivate sopra con i vincoli/pattern già esistenti
nel progetto, nessuna alternativa reale da confermare.

## Test da scrivere

Nessuno, zero modifiche Rust/core. Coerente con l'assenza di un
runner JS per componenti nel progetto.

## Verifica

`npm run typecheck`, `cargo test`, `cargo clippy --all-targets -D
warnings`, `cargo fmt --check` — tutti puliti (zero modifiche Rust, un
file TSX + `App.tsx` + CSS toccati). Non verificato in questa sessione
con un giro reale in `npm run tauri dev`: digitare "scorciatoie" (deve
mostrare ⌘/ accanto a "Mostra scorciatoie" e il match evidenziato),
digitare una stringa senza risultati (deve mostrare "Nessun
risultato"), digitare l'inizio del titolo di una pagina recente (match
evidenziato) — lasciato all'utente.
