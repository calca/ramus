// Riconoscimento visivo dei task nei blocchi: un blocco il cui content
// inizia con "[ ] "/"[x] "/"[X] " è un task, sintassi Obsidian/GFM già
// compatibile — vedi specs/M4/2026-09-02-task-todo-done.TODO.md. Solo
// decorazione, stesso principio di linkTagHighlight.ts: zero modifiche al
// modello dati o al parser.

import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

import { TASK_PATTERN } from "./taskActions";

export const TaskHighlight = Extension.create({
  name: "taskHighlight",

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("taskHighlight"),
        props: {
          decorations(state) {
            const decorations: Decoration[] = [];
            state.doc.descendants((node, pos) => {
              if (node.type.name !== "listItem") {
                return true;
              }
              const paragraph = node.firstChild;
              if (!paragraph) {
                return true;
              }
              const match = TASK_PATTERN.exec(paragraph.textContent);
              if (!match) {
                return true;
              }
              // +1 entra nel listItem (dove inizia il paragraph), +1
              // entra nel paragraph (dove inizia il testo).
              const markerStart = pos + 2;
              decorations.push(
                Decoration.inline(markerStart, markerStart + match[0].length, {
                  class: "editor-task-marker",
                }),
              );
              const isDone = match[1] === "x" || match[1] === "X";
              if (isDone) {
                decorations.push(Decoration.node(pos, pos + node.nodeSize, { class: "task-done" }));
              }
              return true;
            });
            return DecorationSet.create(state.doc, decorations);
          },
        },
      }),
    ];
  },
});
