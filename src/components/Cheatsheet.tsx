import { useTranslation } from "react-i18next";

import { SHORTCUT_ACTIONS, formatShortcut, getShortcut } from "../lib/shortcut";
import type { Config } from "../lib/types";
import { Modal } from "./Modal";

interface CheatsheetProps {
  config: Config;
  onClose: () => void;
}

/** Elenco statico, sola lettura — Tab/Shift-Tab/Invio/Backspace vivono
 * nella keymap ProseMirror dell'editor, non nel registro configurabile
 * (vedi src/lib/shortcut.ts). `labelKey` invece di un'etichetta già
 * tradotta, stesso motivo di SHORTCUT_ACTIONS in src/lib/shortcut.ts:
 * risolta con t() al render, non congelata una volta sola al modulo. */
const EDITOR_SHORTCUTS: { labelKey: string; keys: string }[] = [
  { labelKey: "cheatsheet.editor.newBlock", keys: "Invio" },
  { labelKey: "cheatsheet.editor.indent", keys: "Tab" },
  { labelKey: "cheatsheet.editor.outdent", keys: "Shift+Tab" },
  { labelKey: "cheatsheet.editor.exitLevel", keys: "Backspace" },
  { labelKey: "cheatsheet.editor.moveUp", keys: "Alt+↑" },
  { labelKey: "cheatsheet.editor.moveDown", keys: "Alt+↓" },
  { labelKey: "cheatsheet.editor.cycleTask", keys: "Mod+Invio" },
];

export function Cheatsheet({ config, onClose }: CheatsheetProps) {
  const { t } = useTranslation();
  return (
    <Modal onClose={onClose} ariaLabel={t("cheatsheet.title")}>
      <header className="settings-panel-header">
        <h2>{t("cheatsheet.title")}</h2>
        <button type="button" onClick={onClose} aria-label={t("common.close")}>
          ✕
        </button>
      </header>

      <section className="settings-section">
        <h3>{t("cheatsheet.section.app")}</h3>
        <ul className="cheatsheet-list">
          {SHORTCUT_ACTIONS.map((action) => (
            <li key={action.id}>
              <span>{t(action.labelKey)}</span>
              <kbd>{formatShortcut(getShortcut(config.shortcuts, action.id))}</kbd>
            </li>
          ))}
        </ul>
      </section>

      <section className="settings-section">
        <h3>{t("cheatsheet.section.editor")}</h3>
        <ul className="cheatsheet-list">
          {EDITOR_SHORTCUTS.map((entry) => (
            <li key={entry.labelKey}>
              <span>{t(entry.labelKey)}</span>
              <kbd>{entry.keys}</kbd>
            </li>
          ))}
        </ul>
      </section>
    </Modal>
  );
}
