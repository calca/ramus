// Scorciatoia configurabile per aprire il pannello di ricerca (SettingsPanel
// la cattura, App.tsx la confronta a ogni keydown globale). Formato
// canonico salvato in Config.search_shortcut: "Mod+K", "Mod+Shift+F" —
// "Mod" è il modificatore primario della piattaforma (Cmd su macOS, Ctrl
// altrove), sempre obbligatorio: senza, qualunque lettera digitata
// normalmente nell'editor aprirebbe il pannello, rompendo la scrittura.

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

/** Rappresentazione leggibile per la UI: "⌘K" su macOS, "Ctrl+K" altrove. */
export function formatShortcut(shortcut: string): string {
  const parts = shortcut.split("+");
  const key = parts[parts.length - 1];
  const mods = parts.slice(0, -1);
  if (IS_MAC) {
    return mods.map((m) => MAC_SYMBOLS[m] ?? m).join("") + key;
  }
  return [...mods.map((m) => OTHER_LABELS[m] ?? m), key].join("+");
}
