// Riconoscimento visivo di [[link]] e #tag: solo decorazione, il testo del
// blocco resta esattamente quello digitato (nessun mark/nodo che
// richiederebbe serializzazione dedicata — stesso principio di
// currentBlockHighlight.ts, qui su tutto il documento invece che sul solo
// blocco a fuoco).

import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

import { slugify } from "../lib/pages";

const LINK_PATTERN = /\[\[([^\]]+)\]\]/g;
const TAG_PATTERN = /#[\w-]+/g;

export const LinkTagHighlight = Extension.create({
  name: "linkTagHighlight",

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("linkTagHighlight"),
        props: {
          decorations(state) {
            const decorations: Decoration[] = [];
            state.doc.descendants((node, pos) => {
              if (!node.isText || !node.text) {
                return;
              }
              const text = node.text;
              for (const match of text.matchAll(LINK_PATTERN)) {
                const from = pos + (match.index ?? 0);
                const title = match[1];
                decorations.push(
                  Decoration.inline(from, from + match[0].length, {
                    class: "editor-link",
                    "data-title": title,
                    "data-slug": slugify(title),
                  }),
                );
              }
              for (const match of text.matchAll(TAG_PATTERN)) {
                const from = pos + (match.index ?? 0);
                decorations.push(
                  Decoration.inline(from, from + match[0].length, { class: "editor-tag" }),
                );
              }
            });
            return DecorationSet.create(state.doc, decorations);
          },
        },
      }),
    ];
  },
});
