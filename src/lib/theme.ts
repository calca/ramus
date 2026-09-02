import type { Theme } from "./types";

/** Applica il tema al documento. "system" rimuove l'override esplicito e
 * lascia decidere a `prefers-color-scheme` (vedi assets/palette.css). */
export function applyTheme(theme: Theme): void {
  if (theme === "system") {
    delete document.documentElement.dataset.theme;
  } else {
    document.documentElement.dataset.theme = theme;
  }
}
