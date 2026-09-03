// Riordino di blocchi fratelli da tastiera (Alt+Su/Giù) — vedi
// specs/M4/2026-09-02-riordino-blocchi-tastiera.TODO.md. Nessun comando
// pronto in prosemirror-schema-list per questo (solo sink/lift/split),
// serve una transazione scritta apposta.

import type { Editor } from "@tiptap/core";
import { Fragment, Slice } from "@tiptap/pm/model";
import { TextSelection } from "@tiptap/pm/state";

export type MoveDirection = "up" | "down";

/** Scambia il listItem a fuoco col fratello precedente/successivo allo
 * stesso livello (sottoalbero incluso), mantenendo il cursore nel blocco
 * spostato. Nessun effetto (ma l'evento resta consumato: niente comandi
 * di default del browser per Alt+freccia) se il blocco è già primo/ultimo
 * fratello. */
export function moveListItem(editor: Editor, direction: MoveDirection): boolean {
  const { state, view } = editor;
  const { $from } = state.selection;

  let depth = $from.depth;
  while (depth > 0 && $from.node(depth).type.name !== "listItem") {
    depth--;
  }
  if (depth === 0) {
    // Nessun listItem antenato: non dovrebbe succedere nell'outliner, ma
    // per sicurezza si lascia cadere l'evento al comportamento di default.
    return false;
  }

  const parent = $from.node(depth - 1);
  const index = $from.index(depth - 1);
  const targetIndex = direction === "up" ? index - 1 : index + 1;
  if (targetIndex < 0 || targetIndex >= parent.childCount) {
    return true;
  }

  const currentNode = $from.node(depth);
  const targetNode = parent.child(targetIndex);
  const currentStart = $from.before(depth);
  const cursorOffset = $from.pos - currentStart;

  // I due fratelli sono contigui nel documento (nessun contenuto fra due
  // figli di bulletList): basta la posizione di uno per derivare l'altra.
  const from = direction === "up" ? currentStart - targetNode.nodeSize : currentStart;
  const to =
    direction === "up"
      ? currentStart + currentNode.nodeSize
      : currentStart + currentNode.nodeSize + targetNode.nodeSize;
  const swapped = direction === "up" ? [currentNode, targetNode] : [targetNode, currentNode];
  const newCurrentStart = direction === "up" ? from : from + targetNode.nodeSize;

  const tr = state.tr.replace(from, to, new Slice(Fragment.from(swapped), 0, 0));
  const cursorPos = Math.min(newCurrentStart + cursorOffset, tr.doc.content.size);
  tr.setSelection(TextSelection.near(tr.doc.resolve(cursorPos)));
  view.dispatch(tr);
  return true;
}
