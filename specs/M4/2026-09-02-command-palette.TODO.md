# Command palette (ricerca + creazione + recenti + azioni)

Stato: proposta, in attesa di conferma.

## Motivazione

Secondo pezzo di M4 (SPEC.md, "UI"). Evolve `SearchPanel` (M2,
`specs/M2/2026-09-02-ricerca-full-text.DONE.md`), già agganciato a
Cmd/Ctrl+K, in una vera command palette: ricerca full-text (invariata),
più creazione pagine, pagine aperte di recente, e azioni dell'app
(comprimi finestra, impostazioni, ecc.).

**Non un secondo pannello**: si estende `SearchPanel` esistente
invece di aggiungere una seconda UI in competizione sulla stessa
scorciatoia — un solo posto dove "cercare/fare qualcosa", coerente con
l'app a vista singola.

## Rinomina (solo frontend, nessun impatto sul formato persistito)

Il componente non si chiama più solo "ricerca": `SearchPanel.tsx` →
`CommandPalette.tsx` (componente `CommandPalette`), classi CSS
`.search-input`/`.search-results`/`.search-result*` →
`.palette-input`/`.palette-results`/`.palette-item*`, `activePanel`
`"search"` → `"palette"` in `App.tsx`.

**Resta invariato** `Config::search_shortcut` (il campo Rust
persistito in `config.json`): rinominarlo romperebbe silenziosamente
la scorciatoia già salvata dagli utenti esistenti (il campo mancante
ricadrebbe sul default — non una perdita di dati, ma un reset non
necessario di una preferenza). Il nome resta accurato quanto basta:
la palette include la ricerca, non ne è disgiunta. Cambia solo
l'etichetta visibile in `SettingsPanel` (sezione "Ricerca" → sezione
con nome più ampio, vedi "Domande aperte").

Il bottone nell'header resta uno dei 3 previsti da
`specs/M4/2026-09-02-header-status-bar.TODO.md` — cambia solo cosa fa
al click (stessa icona 🔍, o una diversa, vedi "Domande aperte").

## Contenuto della palette, per stato della query

Lista unica (`<ul>`), navigabile con frecce/Invio come oggi, ma
raggruppata in sezioni con una piccola etichetta muta sopra ciascuna
(non un cambio di componente, solo raggruppamento visivo — stesso
principio di leggerezza già usato altrove, es. `.settings-section h3`).

### Query vuota

- **Recenti**: le pagine aperte di recente (vedi sotto), più recente
  prima.
- **Azioni**: la lista fissa di azioni (vedi sotto), tutte, non
  filtrate.

Nessuna ricerca full-text lanciata a vuoto (comportamento già presente
in `SearchPanel`, invariato: niente chiamata a `search("")`).

### Query non vuota

Tre fonti unite nello stesso ordine di priorità (le azioni prima,
sono rare e ad alta intenzionalità quando combaciano):

1. **Azioni** il cui nome contiene la query (sottostringa,
   case-insensitive — stesso criterio già usato per `listPages`/
   `listTags` negli altri autocomplete).
2. **Risultati di ricerca full-text** (`search(query)`, invariato da
   M2 — stesso comando, stesso rendering con snippet HTML).
3. **"Crea «query»"**, in fondo, solo se nessuna pagina esistente ha
   quel titolo esatto (case-insensitive) — stessa condizione già usata
   in `linkAutocomplete.ts` (`fetchCandidates`), stessa chiamata a
   `listPages()` per verificarlo. Selezionarla chiama
   `navigateToPage(query)`, che già gestisce la creazione (`openPage`)
   — nessuna nuova logica di creazione, solo un nuovo punto d'accesso
   allo stesso flusso.

## Pagine aperte di recente

Nuovo stato in `App.tsx`: `recentPages: string[]` (titoli, più
recente in testa, capped a 10, deduplicato — riaprire una pagina già
in lista la sposta in cima invece di duplicarla). Aggiornato dentro
`navigateToPage`, dopo un `openPage` riuscito.

**Solo pagine**, non giorni di journal: coerente con la richiesta
originale ("pagine aperte di recente") e col fatto che il journal non
si "apre" come azione discreta — si scorre, è sempre presente.

**Non persistito** tra riavvii (sessione corrente soltanto) — stesso
trattamento già scelto per `isCompact` in `App.tsx` ("Non persistita:
è una preferenza di sessione"), stesso principio qui: è una comodità
di navigazione nella sessione attuale, non uno stato che ha senso
salvare su disco. Vedi "Domande aperte" se si preferisce persisterlo.

## Azioni

Lista fissa, non configurabile (coerente con l'assenza di
configurabilità estesa altrove nell'app), definita in un piccolo
modulo nuovo `src/lib/paletteActions.ts`:

```ts
export interface PaletteAction {
  id: string;
  label: string;
  run: () => void;
}
```

Azioni proposte per la v1 (tutte già esistenti come funzioni in
`App.tsx`, la palette è solo un nuovo punto d'accesso):

- **Vai a oggi** (`scrollToToday`) — solo se `view.kind === "journal"`.
- **Torna al journal** (`returnToJournal`) — solo se
  `view.kind === "page"`.
- **Comprimi finestra** / **Espandi finestra** (`toggleCompact`) —
  etichetta dipende da `isCompact`, stesso pattern del bottone
  nell'header.
- **Impostazioni** (`setActivePanel("settings")`)
- **Informazioni su Ramus** (`setActivePanel("about")`)

Costruite dinamicamente (non un array statico puro, alcune sono
condizionali sullo stato corrente) da una funzione
`buildActions(ctx): PaletteAction[]` chiamata a ogni apertura della
palette. Cinque azioni, deliberatamente poche: si aggiungono solo
quelle a cui serve davvero un accesso rapido da tastiera, non un
elenco esaustivo di tutto ciò che l'app sa fare.

## Modello dati della lista

```ts
type PaletteItem =
  | { kind: "action"; action: PaletteAction }
  | { kind: "recent"; title: string }
  | { kind: "hit"; hit: SearchHit }       // riuso di SearchHit, M2
  | { kind: "create"; title: string };
```

Selezione: `action` → `item.action.run()` e chiusura; `recent`/`hit`
di tipo pagina → `navigateToPage(title)`; `hit` di tipo journal →
stessa logica già in `handleSearchSelect` (jump al giorno); `create`
→ `navigateToPage(title)` (crea se manca, invariato).

## Fuori scope per questa spec

- Azioni personalizzabili dall'utente (aggiungere/rimuovere/riordinare
  la lista): fissa per ora.
- Fuzzy matching / ranking avanzato: sottostringa case-insensitive,
  stesso criterio già in uso ovunque nell'app — nessuna libreria di
  fuzzy-search nuova (coerente con CLAUDE.md, niente dipendenza senza
  motivo forte).
- Persistenza dei "recenti" fra riavvii: sessione soltanto (vedi
  "Domande aperte").
- Azioni che aprono un sotto-flusso con altri campi da compilare (es.
  "Cambia vault", che oggi richiede una dialog nativa + conferma): non
  ci sono candidate simili nella lista v1, ma se emergesse in futuro
  andrebbe valutata a parte, non forzata nella stessa palette a riga
  singola.

## Domande aperte

1. **Persistenza dei recenti**: sessione soltanto (proposto, coerente
   con `isCompact`) — o preferisci che sopravvivano al riavvio (in tal
   caso serve deciderne dove: un nuovo campo `Config`, o uno storage
   separato lato frontend)?
2. **Etichetta della sezione in Impostazioni** (oggi "Ricerca"): resta
   "Ricerca" (la scorciatoia in fondo apre comunque prima di tutto una
   ricerca), o si rinomina in qualcosa come "Command palette" /
   "Comandi"?
3. **Icona/aria-label del bottone nell'header**: resta 🔍 "Cerca"
   (proposto, minima discontinuità visiva) o si cambia per riflettere
   lo scope più ampio (es. "Comandi", icona diversa)?
4. Elenco azioni v1 proposto sopra (5 voci) — mancano voci importanti,
   o è già completo per iniziare?

## Test da scrivere

Nessuno lato core: tutta la logica nuova è frontend (nessuna funzione
`ramus-core` coinvolta oltre ai command già esistenti e testati:
`search`, `list_pages`, `open_page`). Nessun test frontend nuovo,
coerente con l'assenza di un runner JS per componenti nel progetto
(stessa scelta già fatta per gli altri autocomplete).

## Verifica

`npm run typecheck` per la parte automatizzabile. L'interazione
(digitare, navigare fra sezioni miste con le frecce, eseguire
un'azione, creare una pagina dalla palette, vedere i recenti
aggiornarsi) non è verificabile in questo sandbox: serve un giro
manuale in `npm run tauri dev`.
