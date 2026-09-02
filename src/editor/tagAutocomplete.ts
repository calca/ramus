// Autocomplete per #tag: stesso @tiptap/suggestion di linkAutocomplete.ts,
// ma senza voce "Crea «query»" — un tag è testo libero, non c'è nulla da
// materializzare su disco (vedi specs/M2/2026-09-02-autocomplete-tag.DONE.md). Se
// nessun tag esistente combacia, il popup semplicemente non appare: il
// testo digitato resta comunque un tag valido, riconosciuto a prescindere
// da linkTagHighlight.ts.

import { Extension } from "@tiptap/core";
import { PluginKey } from "@tiptap/pm/state";
import { ReactRenderer } from "@tiptap/react";
import Suggestion from "@tiptap/suggestion";

import { TagSuggestionList, type TagSuggestionListHandle } from "../components/TagSuggestionList";
import { listTags } from "../lib/commands";
import { positionSuggestionPopup } from "./suggestionPopup";

const MAX_SUGGESTIONS = 8;

// Chiave distinta da quella di linkAutocomplete.ts: vedi il commento lì,
// stesso motivo (default condiviso di @tiptap/suggestion altrimenti in
// collisione fra le due estensioni sullo stesso editor).
const TAG_SUGGESTION_KEY = new PluginKey("tagAutocomplete");

async function fetchCandidates(query: string): Promise<string[]> {
  const tags = await listTags();
  const q = query.trim().toLowerCase();
  const filtered = q ? tags.filter((tag) => tag.toLowerCase().includes(q)) : tags;
  return filtered.slice(0, MAX_SUGGESTIONS);
}

export const TagAutocomplete = Extension.create({
  name: "tagAutocomplete",

  addProseMirrorPlugins() {
    return [
      Suggestion<string, string>({
        pluginKey: TAG_SUGGESTION_KEY,
        editor: this.editor,
        char: "#",
        items: ({ query }) => fetchCandidates(query),
        command: ({ editor, range, props: tag }) => {
          editor.chain().focus().insertContentAt(range, `#${tag}`).run();
        },
        render: () => {
          let component: ReactRenderer<TagSuggestionListHandle> | null = null;

          return {
            onStart: (props) => {
              component = new ReactRenderer(TagSuggestionList, {
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
