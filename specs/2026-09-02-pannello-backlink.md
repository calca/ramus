# Pannello backlink sulla pagina aperta

Stato: implementata come descritto, inclusa la scelta di lasciare i
backlink da journal non cliccabili (unico punto di "Domande aperte",
nessuna obiezione ricevuta — si procede con la proposta di default).

## Motivazione

Terzo pezzo di M2 (SPEC.md): "Pannello backlink sulla pagina aperta".
`specs/2026-09-02-navigazione-pagine.md` aveva già rimandato
esplicitamente questa parte in attesa dell'indice SQLite ("Pannello
backlink vero e proprio... è esattamente il lavoro dell'indice SQLite —
spec a parte, dopo questa"). L'indice è implementato
(`specs/2026-09-02-indice-sqlite.md`) ed espone già
`Index::find_backlinks` / il command Tauri `find_backlinks` e il
wrapper frontend `findBacklinks`, tutti inutilizzati finora. Questa
spec è il primo consumo.

## Dove vive

Solo in `PageView.tsx` (mai nella vista journal): un giorno di journal
non è oggi un target di `[[link]]` valido (deciso fuori scope in
`specs/2026-09-02-link-tag-parsing.md`), quindi non può avere
backlink in entrata. Sezione nuova sotto l'editor, dentro lo stesso
`.page-view-content` centrato (coerente con l'allineamento già corretto
in `specs/2026-09-02-navigazione-pagine.md` — niente contenitore
separato con la sua larghezza).

```tsx
<div className="page-view-content">
  <button className="page-view-back">...</button>
  <h1 className="page-view-title">...</h1>
  <Editor ... />
  <BacklinksSection backlinks={backlinks} onSelect={onLinkClick} />
</div>
```

## Che titolo si usa per cercare i backlink

`find_backlinks(target_title)` confronta via `slugify` (vedi
`crates/ramus-core/src/index.rs`), quindi basta passare il titolo della
pagina aperta. Fallback identico a quello già usato per l'intestazione
(`page.title ?? page.path`) **non va bene qui**: se `page.title` è
`None` (pagina senza front-matter, caso limite non raggiungibile dal
flusso normale dell'app — `open_page` scrive sempre un front-matter),
passare l'intero path (`"pages/x.md"`) a `find_backlinks` produrrebbe
uno slug sbagliato (contiene `/` e `.`). Fallback corretto: lo slug
derivato dal path stesso (`page.path` meno il prefisso `pages/` e il
suffisso `.md`, già uno slug valido per costruzione — vedi
`Vault::page_relative_path`). Nella pratica quasi mai esercitato.

```ts
function backlinkTarget(page: Page): string {
  if (page.title) return page.title;
  return page.path.replace(/^pages\//, "").replace(/\.md$/, "");
}
```

## `PageView.tsx`

- Nuovo `useEffect` su `page.path`: chiama `findBacklinks(backlinkTarget(page))`,
  salva il risultato in `useState<Backlink[]>([])`. Un solo fetch al
  montaggio/cambio pagina, nessun refresh in tempo reale: i backlink
  riguardano cosa linkano **altre** pagine verso questa, e mentre
  `PageView` è aperta l'utente non può modificare altre pagine in
  parallelo (la vista journal resta montata ma non visibile/a fuoco —
  vedi `specs/2026-09-02-navigazione-pagine.md`). Un link aggiunto
  altrove diventa visibile alla prossima apertura di questa pagina,
  già garantito perché `write_page` chiama `Index::refresh_page` prima
  di ogni ritorno (comportamento esistente, nessuna modifica).
- Stato di caricamento: nessuno stile dedicato — la query è locale
  (SQLite su disco, nessuna rete), il fetch è già completo prima che
  l'utente noti qualcosa nella grandissima maggioranza dei casi.
  Nessuno spinner, coerente con la sobrietà del resto dell'app.

## `BacklinksSection` (nuovo componente, `src/components/BacklinksSection.tsx`)

```tsx
interface BacklinksSectionProps {
  backlinks: Backlink[];
  onSelect: (title: string) => void;
}
```

- Lista vuota: nessuna sezione visibile (niente "0 backlink" — se non
  c'è nulla da mostrare, non si mostra nulla, stesso principio già
  seguito altrove nell'app per stati vuoti che non richiedono azione).
- Lista non vuota: intestazione "Backlink" (`<h2>`, stesso stile di un
  futuro titolo di sezione — non esiste ancora un `<h2>` altrove
  nell'app: nuova classe `.page-view-backlinks h2`, dimensione/peso
  simile a `.settings-section h3` ma non identica, è un titolo di primo
  livello nella pagina, non un sotto-titolo di pannello) più un elenco.
- Ogni voce mostra:
  - **Da dove**: se `source_path` inizia con `journals/`, la data
    (`2026-09-01`, estratta dal filename — stesso schema usato altrove,
    es. `journalDateFromPath`); altrimenti `source_title ?? source_path`.
  - **Cosa**: `block_content` per intero (i blocchi sono tipicamente
    corti — una riga di outliner, non un paragrafo lungo — nessun
    troncamento necessario per ora).
- Interazione al click su una voce:
  - Sorgente una **pagina**: chiama `onSelect(source_title ?? <slug
    dal source_path>)` — stessa funzione `navigateToPage` già passata
    come `onLinkClick` a `Editor`, **riusata**, non duplicata: aprire
    un backlink verso una pagina è la stessa azione di cliccare un
    `[[link]]`.
  - Sorgente un **journal**: **non cliccabile** in questa spec (vedi
    "Fuori scope") — la vista di un singolo giorno del journal fuori
    dalla lista verticale principale non esiste come concetto
    navigabile isolato. Il testo resta comunque leggibile (la data),
    solo senza azione al click, stile visivo neutro invece che
    `--ramus-sap` (che nell'app indica sempre "questo è cliccabile",
    vedi CSS di `.editor-link`/`.page-view-back`).

## CSS (`src/index.css`)

```css
.page-view-backlinks {
  margin-top: 2.5rem;
  padding-top: 1.5rem;
  border-top: 1px solid color-mix(in srgb, var(--ramus-stone) 20%, transparent);
}

.page-view-backlinks h2 {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--ramus-stone);
  margin: 0 0 0.75rem;
}

.backlink-item {
  padding: 0.5rem 0;
}

.backlink-item + .backlink-item {
  border-top: 1px solid color-mix(in srgb, var(--ramus-stone) 12%, transparent);
}

.backlink-item-source {
  font-size: 0.8rem;
  color: var(--ramus-stone);
  margin: 0 0 0.15rem;
}

.backlink-item-source.is-clickable {
  color: var(--ramus-sap);
  cursor: pointer;
  background: none;
  border: none;
  padding: 0;
  font: inherit;
}

.backlink-item-content {
  font-size: 0.9rem;
  margin: 0;
}
```

## Fuori scope per questa spec

- Backlink da un journal verso una pagina, cliccabile fino al giorno
  esatto: serve prima un modo di navigare/evidenziare un giorno
  specifico del journal isolatamente — collegato a
  "Domande aperte" della prossima spec (ricerca full-text), che ha lo
  stesso identico problema per i risultati di tipo journal. Se lì si
  decide una soluzione (es. riuso di `jumpToDate` esistente in
  `App.tsx`), si può applicare anche qui in un secondo momento senza
  cambiare lo schema dati.
- Backlink verso un **journal** (linkare `[[2026-09-01]]`): non esiste
  ancora come sintassi valida (fuori scope già in
  `specs/2026-09-02-link-tag-parsing.md`), quindi non può esistere nel
  pannello di un journal — coerente col fatto che la sezione non
  esiste affatto nella vista journal.
- Contare/mostrare quante volte una pagina è linkata (badge numerico
  nell'header o nella lista `list_pages`): non richiesto da SPEC.md per
  M2, estensione piccola ma separata se serve.
- Anteprima con evidenziazione del testo del link dentro
  `block_content` (es. `<b>[[Questa Pagina]]</b>`): il blocco viene
  mostrato per intero, senza markup aggiuntivo — un'anteprima
  evidenziata è la stessa tecnica della prossima spec (snippet di
  ricerca via tantivy), valutabile insieme più avanti se serve qui
  troppo.

## Domande aperte

Nessuna bloccante — un solo punto di conferma: va bene che i backlink
da un **journal** restino testo non cliccabile per ora (vedi sopra),
invece di aspettare che esista prima una vista/scroll-to-day dedicata?
L'alternativa è rimandare l'intera spec finché quella capacità non
esiste, ma i backlink da pagina (il caso più comune per un uso tipico
tipo "Progetti" o "Persone") sarebbero comunque utili subito.

## Test da scrivere

Nessuno lato core: `find_backlinks` è già testato in
`crates/ramus-core/src/index.rs`. Nessun test frontend nuovo, coerente
con l'assenza di un runner JS per componenti nel progetto (vedi la
spec dell'autocomplete tag).

## Verifica

`npm run typecheck`. L'interazione (aprire una pagina linkata da più
punti, verificare che tutte le fonti compaiano, cliccare un backlink da
pagina e verificare la navigazione) non è verificabile in questo
sandbox: serve un giro manuale in `npm run tauri dev`.
