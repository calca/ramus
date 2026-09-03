import { SHORTCUT_ACTIONS, formatShortcut, getShortcut } from "../lib/shortcut";
import type { Config } from "../lib/types";
import { Modal } from "./Modal";

interface CheatsheetProps {
  config: Config;
  onClose: () => void;
}

/** Elenco statico, sola lettura — Tab/Shift-Tab/Invio/Backspace vivono
 * nella keymap ProseMirror dell'editor, non nel registro configurabile
 * (vedi src/lib/shortcut.ts). */
const EDITOR_SHORTCUTS: { label: string; keys: string }[] = [
  { label: "Nuovo blocco", keys: "Invio" },
  { label: "Indenta", keys: "Tab" },
  { label: "Rimuovi indentazione", keys: "Shift+Tab" },
  { label: "Esci di un livello (blocco vuoto)", keys: "Backspace" },
  { label: "Sposta blocco su", keys: "Alt+↑" },
  { label: "Sposta blocco giù", keys: "Alt+↓" },
];

export function Cheatsheet({ config, onClose }: CheatsheetProps) {
  return (
    <Modal onClose={onClose} ariaLabel="Scorciatoie">
      <header className="settings-panel-header">
        <h2>Scorciatoie</h2>
        <button type="button" onClick={onClose} aria-label="Chiudi">
          ✕
        </button>
      </header>

      <section className="settings-section">
        <h3>App</h3>
        <ul className="cheatsheet-list">
          {SHORTCUT_ACTIONS.map((action) => (
            <li key={action.id}>
              <span>{action.label}</span>
              <kbd>{formatShortcut(getShortcut(config.shortcuts, action.id))}</kbd>
            </li>
          ))}
        </ul>
      </section>

      <section className="settings-section">
        <h3>Editor</h3>
        <ul className="cheatsheet-list">
          {EDITOR_SHORTCUTS.map((entry) => (
            <li key={entry.label}>
              <span>{entry.label}</span>
              <kbd>{entry.keys}</kbd>
            </li>
          ))}
        </ul>
      </section>
    </Modal>
  );
}
