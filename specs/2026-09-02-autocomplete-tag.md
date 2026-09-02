# Autocomplete per `#tag`

Stato: implementata. `positionPopup` estratto in
`src/editor/suggestionPopup.ts` come previsto, riusato sia da
`linkAutocomplete.ts` sia dal nuovo `tagAutocomplete.ts`.

## Motivazione

Debito esplicitamente rimandato in `specs/2026-09-02-link-tag-parsing.md`
("`#tag`: solo riconoscimento, nessun autocomplete... Meglio aspettare
l'indice: il riconoscimento visivo di `#tag` resta comunque, solo il
popup di suggerimenti arriva dopo"). L'indice SQLite
(`specs/2026-09-02-indice-sqlite.md`) è implementato e già espone
`Index::list_tags` / il command Tauri `list_tags`, inutilizzati finora.
Questa spec è il primo consumo di quel command.

## Differenza principale rispetto all'autocomplete dei link

Un `[[link]]` selezionato "crea" concettualmente una pagina (file su
disco) se non esiste. Un `#tag` no: è testo libero, esiste nel momento
in cui viene digitato — non c'è nulla da materializzare. Questo
semplifica il flusso rispetto a `linkAutocomplete.ts`:

- Nessuna voce "Crea «query»": la lista mostra solo i tag già esistenti
  nel vault che combaciano col testo digitato.
- Se la lista è vuota (nessun tag esistente combacia, o il vault non ha
  ancora nessun tag), il popup non appare — il testo digitato resta
  comunque un tag valido (la decorazione `.editor-tag` di
  `linkTagHighlight.ts` lo riconosce a prescindere dal popup, non
  cambia nulla qui).
- Nessuna chiamata a un equivalente di `open_page`: selezionare un
  suggerimento sostituisce solo il testo digitato con `#{tag}`.

## Implementazione

### `src/editor/tagAutocomplete.ts` (nuovo)

Stesso pattern di `linkAutocomplete.ts` (`@tiptap/suggestion`, già
dipendenza esistente — nessuna nuova dipendenza), trigger `"#"` invece
di `"[["`:

```ts
const MAX_SUGGESTIONS = 8;

async function fetchCandidates(query: string): Promise<string[]> {
  const tags = await listTags();
  const q = query.trim().toLowerCase();
  const filtered = q ? tags.filter((t) => t.toLowerCase().includes(q)) : tags;
  return filtered.slice(0, MAX_SUGGESTIONS);
}

Suggestion<string, string>({
  editor: this.editor,
  char: "#",
  items: ({ query }) => fetchCandidates(query),
  command: ({ editor, range, props: tag }) => {
    editor.chain().focus().insertContentAt(range, `#${tag}`).run();
  },
  render: () => { /* stesso scheletro onStart/onUpdate/onKeyDown/onExit
                      di linkAutocomplete.ts, stesso positionPopup() */ },
})
```

`positionPopup` viene estratto da `linkAutocomplete.ts` in un piccolo
modulo condiviso (`src/editor/suggestionPopup.ts`) invece di essere
duplicato: unica logica di posizionamento, usata da entrambe le
estensioni.

### `src/components/TagSuggestionList.tsx` (nuovo)

Quasi identico a `LinkSuggestionList.tsx` ma su `string[]` invece di
`LinkCandidate[]` (niente distinzione `kind`):

```tsx
interface TagSuggestionListProps {
  items: string[];
  command: (tag: string) => void;
}
```

Stesso `useImperativeHandle` per `onKeyDown` (frecce, invio), stesso
markup (`.link-suggestion-list`/`.link-suggestion-item` **riusati**,
non duplicati in CSS — è lo stesso popup visivo, cambia solo il
contenuto testuale delle righe).

### Registrazione

`src/editor/extensions.ts`: aggiunta `TagAutocomplete` alla lista in
`createExtensions()`, dopo `LinkAutocomplete`.

### `char: "#"` e conflitto con headings

`StarterKit` ha `heading: false` (SPEC.md M1): `#` non ha già un
significato speciale nell'editor, nessun conflitto da risolvere.

### Perché non serve gestire "nessun match esatto"

A differenza dei link (dove serve sapere se creare o no un file),
qui non c'è alcuna azione da compiere se l'utente ignora il popup e
continua a digitare: il testo `#nuovotag` resta semplicemente testo,
già colorato correttamente da `linkTagHighlight.ts` (regex
indipendente dal popup). Il popup è solo un acceleratore per tag già
usati altrove nel vault, non un gate.

## Limite noto e accettato

I tag mostrati nel popup vengono da `list_tags()`, che legge
dall'indice SQLite — riflette solo contenuto già **salvato su disco**
(il debounce di 500ms di `Editor.tsx` scrive dopo una pausa nella
digitazione). Un tag appena digitato nello stesso blocco non ancora
salvato non compare fra i suggerimenti finché non viene scritto: stesso
limite già accettato per `list_pages()` nell'autocomplete dei link.

## Fuori scope

- Tag "che iniziano con" vs "che contengono" la query: si usa
  `includes` (sottostringa), stesso criterio di `linkAutocomplete.ts`
  per i titoli pagina — coerenza fra i due popup.
- Rinominare un tag ovunque nel vault (find & replace): nessuna UI per
  farlo.
- Elenco/browser di tutti i tag del vault (una vista dedicata): non è
  questa spec, eventualmente un'estensione piccola sopra `list_tags`.

## Test da scrivere

Nessuno lato core: `list_tags` è già testato in
`crates/ramus-core/src/index.rs`. Lato frontend, `fetchCandidates`
(filtro case-insensitive, limite a `MAX_SUGGESTIONS`) può avere un test
unitario se si introduce un runner JS — oggi il progetto non ne ha uno
per `src/editor/*`, coerente con `linkAutocomplete.ts` che non ne ha:
non se ne aggiunge uno solo per questa spec.

## Verifica

`npm run typecheck`. L'interazione (digitare `#`, filtrare, selezionare
con tastiera o click) non è verificabile in questo sandbox: serve un
giro manuale in `npm run tauri dev`.
