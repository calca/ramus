// Posizionamento del popup di un'estensione @tiptap/suggestion (link, tag):
// niente tippy.js, un div position:fixed ancorato al clientRect() del
// trigger basta — una dipendenza in meno. Condiviso fra linkAutocomplete.ts
// e tagAutocomplete.ts, stessa logica per entrambi i popup.

export function positionSuggestionPopup(
  element: HTMLElement,
  clientRect?: (() => DOMRect | null) | null,
) {
  const rect = clientRect?.();
  if (!rect) {
    return;
  }
  element.style.position = "fixed";
  element.style.left = `${rect.left}px`;
  element.style.top = `${rect.bottom + 4}px`;
}
