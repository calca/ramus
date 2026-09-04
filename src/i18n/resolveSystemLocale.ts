// Rilevazione "segue il sistema" scritta a mano (non
// i18next-browser-languagedetector, vedi
// specs/M7/2026-09-04-i18n-interfaccia.DONE.md): solo due lingue supportate,
// un pacchetto dedicato alla sola rilevazione da browser storage non serve.
// Stesso principio di IS_MAC in src/lib/shortcut.ts.

/** Legge `navigator.language` e ritorna `"it"` se inizia per `"it"`,
 * altrimenti `"en"` — fallback universale per chiunque non sia italiano,
 * non un tentativo di coprire ogni locale possibile. */
export function resolveSystemLocale(): "it" | "en" {
  const language = typeof navigator !== "undefined" ? navigator.language : "";
  return language.toLowerCase().startsWith("it") ? "it" : "en";
}
