// Il blocco in focus è, insieme al giorno corrente, l'unico altro uso
// consentito dell'accento amber (SPEC.md — Palette). Qui si marca con una
// decorazione ProseMirror il <li> che contiene il cursore, così lo stile
// vive in CSS (vedi .block-focused in index.css) invece che in JS.

import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

export const CurrentBlockHighlight = Extension.create({
  name: "currentBlockHighlight",

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("currentBlockHighlight"),
        props: {
          decorations(state) {
            const { $from } = state.selection;
            for (let depth = $from.depth; depth > 0; depth -= 1) {
              const node = $from.node(depth);
              if (node.type.name === "listItem") {
                const pos = $from.before(depth);
                return DecorationSet.create(state.doc, [
                  Decoration.node(pos, pos + node.nodeSize, { class: "block-focused" }),
                ]);
              }
            }
            return DecorationSet.empty;
          },
        },
      }),
    ];
  },
});
