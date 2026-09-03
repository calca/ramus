// Configurazione Tiptap per l'outliner a blocchi.
//
// StarterKit: si tengono solo bulletList/listItem come struttura a blocchi.
// heading/blockquote/codeBlock/horizontalRule sono disattivati per la
// milestone 1 (SPEC.md). orderedList e hardBreak sono disattivati anche
// loro: il formato su disco non distingue liste ordinate (solo "- "), e un
// hard break introdurrebbe un newline dentro un singolo blocco, violando
// "un blocco = una riga". Bold/italic sono attivi (scorciatoie Mod-B/Mod-I
// di default di StarterKit, invariate) — serializzati a mano in
// editor/inlineMarks.ts, vedi
// specs/refinement/2026-09-03-testo-grassetto-corsivo.DONE.md. Strike e
// code restano disattivati (fuori scope per ora, non serializzabili).
//
// Tab/Shift-Tab/Enter arrivano già corretti dai default di ListItem
// (sinkListItem/liftListItem/splitListItem). Backspace su blocco vuoto è
// l'unico comportamento non coperto dai default e va aggiunto qui.

import { Extension } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";

import { CurrentBlockHighlight } from "./currentBlockHighlight";
import { LinkAutocomplete } from "./linkAutocomplete";
import { LinkTagHighlight } from "./linkTagHighlight";
import { moveListItem } from "./moveBlock";
import { TagAutocomplete } from "./tagAutocomplete";
import { cycleTaskState } from "./taskActions";
import { TaskHighlight } from "./taskHighlight";

const OutlinerBackspace = Extension.create({
  name: "outlinerBackspace",

  addKeyboardShortcuts() {
    return {
      Backspace: ({ editor }) => {
        const { $from, empty } = editor.state.selection;
        const atStartOfEmptyBlock = empty && $from.parentOffset === 0 && $from.parent.textContent.length === 0;
        if (!atStartOfEmptyBlock) {
          // Non è un blocco vuoto: si affida al comportamento di default
          // dell'editor (cancellazione carattere per carattere / merge).
          return false;
        }
        // Blocco vuoto: prima si esce di un livello (lift). Se non si può
        // più uscire (già al livello radice), si lascia cadere l'evento
        // al comportamento di default, che esegue il merge col precedente.
        return editor.commands.liftListItem("listItem");
      },
    };
  },
});

const MoveBlock = Extension.create({
  name: "moveBlock",

  addKeyboardShortcuts() {
    return {
      "Alt-ArrowUp": ({ editor }) => moveListItem(editor, "up"),
      "Alt-ArrowDown": ({ editor }) => moveListItem(editor, "down"),
    };
  },
});

const TaskCycle = Extension.create({
  name: "taskCycle",

  addKeyboardShortcuts() {
    return {
      "Mod-Enter": ({ editor }) => cycleTaskState(editor),
    };
  },
});

export function createExtensions() {
  return [
    StarterKit.configure({
      heading: false,
      blockquote: false,
      codeBlock: false,
      horizontalRule: false,
      orderedList: false,
      hardBreak: false,
      strike: false,
      code: false,
    }),
    OutlinerBackspace,
    MoveBlock,
    TaskCycle,
    CurrentBlockHighlight,
    LinkTagHighlight,
    TaskHighlight,
    LinkAutocomplete,
    TagAutocomplete,
  ];
}
