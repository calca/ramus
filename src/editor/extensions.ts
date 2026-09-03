// Configurazione Tiptap per l'outliner a blocchi.
//
// StarterKit: si tengono solo bulletList/listItem come struttura a blocchi.
// heading/blockquote/codeBlock/horizontalRule sono disattivati per la
// milestone 1 (SPEC.md). orderedList e hardBreak sono disattivati anche
// loro: il formato su disco non distingue liste ordinate (solo "- "), e un
// hard break introdurrebbe un newline dentro un singolo blocco, violando
// "un blocco = una riga". I marks inline (bold/italic/strike/code) sono
// disattivati: il contenuto di un blocco è markdown grezzo come testo
// semplice, non richiede un serializer di marks -> sintassi markdown.
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

export function createExtensions() {
  return [
    StarterKit.configure({
      heading: false,
      blockquote: false,
      codeBlock: false,
      horizontalRule: false,
      orderedList: false,
      hardBreak: false,
      bold: false,
      italic: false,
      strike: false,
      code: false,
    }),
    OutlinerBackspace,
    MoveBlock,
    CurrentBlockHighlight,
    LinkTagHighlight,
    LinkAutocomplete,
    TagAutocomplete,
  ];
}
