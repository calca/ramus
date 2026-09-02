// Autocomplete per [[link]]: @tiptap/suggestion per trigger-detection e
// navigazione da tastiera (motivato in specs/2026-09-02-link-tag-parsing.md
// — superficie fragile da reimplementare a mano), popup posizionato a
// mano via clientRect() (niente tippy.js: un div position:fixed basta,
// una dipendenza in meno).

import { Extension } from "@tiptap/core";
import { PluginKey } from "@tiptap/pm/state";
import { ReactRenderer } from "@tiptap/react";
import Suggestion from "@tiptap/suggestion";

import {
  LinkSuggestionList,
  type LinkCandidate,
  type LinkSuggestionListHandle,
} from "../components/LinkSuggestionList";
import { listPages, openPage } from "../lib/commands";
import { positionSuggestionPopup } from "./suggestionPopup";

const MAX_SUGGESTIONS = 8;

// @tiptap/suggestion usa una PluginKey condivisa di default ("suggestion"):
// due estensioni Suggestion sullo stesso editor senza chiave esplicita
// collidono ("Adding different instances of a keyed plugin"). Serve una
// chiave distinta per estensione — vedi anche tagAutocomplete.ts.
const LINK_SUGGESTION_KEY = new PluginKey("linkAutocomplete");

async function fetchCandidates(query: string): Promise<LinkCandidate[]> {
  const pages = await listPages();
  const trimmed = query.trim();
  const q = trimmed.toLowerCase();
  const filtered = q ? pages.filter((p) => p.title.toLowerCase().includes(q)) : pages;
  const candidates: LinkCandidate[] = filtered
    .slice(0, MAX_SUGGESTIONS)
    .map((p) => ({ kind: "existing", title: p.title }));

  const exactMatch = pages.some((p) => p.title.toLowerCase() === q);
  if (trimmed && !exactMatch) {
    candidates.push({ kind: "create", title: trimmed });
  }
  return candidates;
}

export const LinkAutocomplete = Extension.create({
  name: "linkAutocomplete",

  addProseMirrorPlugins() {
    return [
      Suggestion<LinkCandidate, LinkCandidate>({
        pluginKey: LINK_SUGGESTION_KEY,
        editor: this.editor,
        char: "[[",
        items: ({ query }) => fetchCandidates(query),
        command: ({ editor, range, props: candidate }) => {
          editor.chain().focus().insertContentAt(range, `[[${candidate.title}]]`).run();
          if (candidate.kind === "create") {
            void openPage(candidate.title);
          }
        },
        render: () => {
          let component: ReactRenderer<LinkSuggestionListHandle> | null = null;

          return {
            onStart: (props) => {
              component = new ReactRenderer(LinkSuggestionList, {
                props: { items: props.items, command: props.command },
                editor: props.editor,
              });
              const element = component.element as HTMLElement;
              positionSuggestionPopup(element, props.clientRect);
              document.body.appendChild(element);
            },
            onUpdate: (props) => {
              component?.updateProps({ items: props.items, command: props.command });
              if (component) {
                positionSuggestionPopup(component.element as HTMLElement, props.clientRect);
              }
            },
            onKeyDown: (props) => {
              if (props.event.key === "Escape") {
                component?.element.remove();
                component?.destroy();
                return true;
              }
              return component?.ref?.onKeyDown(props) ?? false;
            },
            onExit: () => {
              component?.element.remove();
              component?.destroy();
              component = null;
            },
          };
        },
      }),
    ];
  },
});
