// Interazione con i task nei blocchi (M4) — click sul marker e ciclo a
// tastiera. Zero modifiche al modello dati: un task è solo testo che
// inizia con "[ ] "/"[x] "/"[X] ", vedi taskHighlight.ts per la
// decorazione e specs/M4/2026-09-02-task-todo-done.TODO.md.

import type { Editor } from "@tiptap/core";
import type { ResolvedPos } from "@tiptap/pm/model";
import { TextSelection } from "@tiptap/pm/state";

export const TASK_PATTERN = /^\[( |x|X)\] /;

/** Risale dalla posizione data al listItem più vicino, o `null` se non
 * ce n'è uno (non dovrebbe succedere nell'outliner). */
function findListItemDepth($pos: ResolvedPos): number | null {
  let depth = $pos.depth;
  while (depth > 0 && $pos.node(depth).type.name !== "listItem") {
    depth--;
  }
  return depth > 0 ? depth : null;
}

/** Click su `.editor-task-marker`: sostituisce solo il carattere di stato
 * (spazio ↔ x), una transazione mirata di un singolo carattere. */
export function toggleTaskMarker(editor: Editor, domNode: Node): boolean {
  const pos = editor.view.posAtDOM(domNode, 0);
  const $pos = editor.state.doc.resolve(pos);
  const depth = findListItemDepth($pos);
  if (depth === null) {
    return false;
  }

  const listItem = $pos.node(depth);
  const paragraph = listItem.firstChild;
  if (!paragraph) {
    return false;
  }
  const match = TASK_PATTERN.exec(paragraph.textContent);
  if (!match) {
    return false;
  }

  // listItem apre a before(depth), +1 entra nel suo contenuto (paragraph),
  // +1 entra nel contenuto del paragraph (il testo), +1 supera "[": qui
  // inizia il carattere di stato.
  const stateCharPos = $pos.before(depth) + 3;
  const nextChar = match[1] === " " ? "x" : " ";
  const tr = editor.state.tr.insertText(nextChar, stateCharPos, stateCharPos + 1);
  editor.view.dispatch(tr);
  return true;
}

/** Scorciatoia Mod-Enter: ciclo a tre stati sul blocco a fuoco (normale →
 * "[ ] " → "[x] " → normale). Il cursore resta alla stessa posizione
 * relativa al testo "vero" del blocco (dopo il marker, se presente). */
export function cycleTaskState(editor: Editor): boolean {
  const { state, view } = editor;
  const { $from } = state.selection;

  const depth = findListItemDepth($from);
  if (depth === null) {
    return false;
  }

  const listItem = $from.node(depth);
  const paragraph = listItem.firstChild;
  if (!paragraph) {
    return false;
  }
  const text = paragraph.textContent;
  const match = TASK_PATTERN.exec(text);

  const textStart = $from.before(depth) + 2;
  const textEnd = textStart + text.length;

  let nextText: string;
  let cursorDelta: number;
  if (!match) {
    nextText = `[ ] ${text}`;
    cursorDelta = 4;
  } else if (match[1] === " ") {
    nextText = `[x] ${text.slice(4)}`;
    cursorDelta = 0;
  } else {
    nextText = text.slice(4);
    cursorDelta = -4;
  }

  const tr = state.tr.insertText(nextText, textStart, textEnd);
  const newCursorPos = Math.max(
    textStart,
    Math.min($from.pos + cursorDelta, tr.doc.content.size),
  );
  tr.setSelection(TextSelection.near(tr.doc.resolve(newCursorPos)));
  view.dispatch(tr);
  return true;
}
