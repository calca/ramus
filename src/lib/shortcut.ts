// Scorciatoie app-level configurabili (SettingsPanel le cattura, App.tsx le
// confronta a ogni keydown globale). Formato canonico salvato in
// Config.shortcuts[id]: "Mod+K", "Mod+Shift+F" — "Mod" è il modificatore
// primario della piattaforma (Cmd su macOS, Ctrl altrove), sempre
// obbligatorio: senza, qualunque lettera digitata normalmente nell'editor
// scatenerebbe l'azione, rompendo la scrittura.
//
// Solo scorciatoie a livello finestra entrano in questo registro — Tab,
// Shift-Tab, Invio, Backspace e il riordino blocchi restano fisse, vivono
// nella keymap ProseMirror dell'editor, non qui (vedi
// specs/M4/2026-09-02-scorciatoie-configurabili.TODO.md, "Cosa resta fuori").

export interface ShortcutAction {
  id: string;
  label: string;
  default: string;
}

export const SHORTCUT_ACTIONS: ShortcutAction[] = [
  { id: "command_palette", label: "Apri command palette", default: "Mod+K" },
  { id: "cheatsheet", label: "Mostra scorciatoie", default: "Mod+/" },
  { id: "open_tasks", label: "Task aperti", default: "Mod+T" },
  { id: "focus_mode", label: "Focus mode (nascondi tutto tranne l'editor)", default: "Mod+." },
  { id: "journal_prev_day", label: "Giorno precedente del journal", default: "Mod+ArrowUp" },
  { id: "journal_next_day", label: "Giorno successivo del journal", default: "Mod+ArrowDown" },
];

/** Legge Config.shortcuts[actionId], ricade sul default del registro se la
 * chiave manca (config scritto prima che l'azione esistesse). */
export function getShortcut(shortcuts: Record<string, string>, actionId: string): string {
  const action = SHORTCUT_ACTIONS.find((a) => a.id === actionId);
  return shortcuts[actionId] ?? action?.default ?? "";
}

const IS_MAC =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform ?? navigator.userAgent);

function hasPrimaryModifier(event: KeyboardEvent): boolean {
  return IS_MAC ? event.metaKey : event.ctrlKey;
}

/** Normalizza un KeyboardEvent in una stringa canonica, o `null` se non è
 * ancora una combinazione valida (manca il modificatore primario, o il
 * tasto premuto è esso stesso un modificatore). */
export function normalizeShortcut(event: KeyboardEvent): string | null {
  if (["Control", "Meta", "Shift", "Alt"].includes(event.key)) {
    return null;
  }
  if (!hasPrimaryModifier(event)) {
    return null;
  }
  const parts = ["Mod"];
  if (event.shiftKey) parts.push("Shift");
  if (event.altKey) parts.push("Alt");
  parts.push(event.key.length === 1 ? event.key.toUpperCase() : event.key);
  return parts.join("+");
}

export function matchesShortcut(event: KeyboardEvent, shortcut: string): boolean {
  const parts = shortcut.split("+");
  const key = parts[parts.length - 1];
  const wantShift = parts.includes("Shift");
  const wantAlt = parts.includes("Alt");
  if (!hasPrimaryModifier(event)) return false;
  if (event.shiftKey !== wantShift) return false;
  if (event.altKey !== wantAlt) return false;
  const eventKey = event.key.length === 1 ? event.key.toUpperCase() : event.key;
  return eventKey === key;
}

const MAC_SYMBOLS: Record<string, string> = { Mod: "⌘", Shift: "⇧", Alt: "⌥" };
const OTHER_LABELS: Record<string, string> = { Mod: "Ctrl", Shift: "Shift", Alt: "Alt" };
const KEY_SYMBOLS: Record<string, string> = {
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};

/** Rappresentazione leggibile per la UI: "⌘K" su macOS, "Ctrl+K" altrove. */
export function formatShortcut(shortcut: string): string {
  const parts = shortcut.split("+");
  const key = KEY_SYMBOLS[parts[parts.length - 1]] ?? parts[parts.length - 1];
  const mods = parts.slice(0, -1);
  if (IS_MAC) {
    return mods.map((m) => MAC_SYMBOLS[m] ?? m).join("") + key;
  }
  return [...mods.map((m) => OTHER_LABELS[m] ?? m), key].join("+");
}
