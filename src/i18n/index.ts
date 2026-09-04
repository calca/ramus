// Istanza i18next dell'app — inizializzata qui, importata da main.tsx
// *prima* del render (useTranslation() nei componenti assume che esista
// già). Riesportata come default export: i moduli non-React (paletteActions.ts,
// lib/shortcut.ts, lib/journal.ts) importano da qui invece che dal pacchetto
// "i18next" direttamente, cosi l'import garantisce che init() sia già
// girato (side effect di modulo, valutato una sola volta — i moduli ES sono
// cache singleton) anche se main.tsx non è ancora stato eseguito (caso dei
// test, che importano i moduli direttamente).
//
// Config.locale (persistita in config.json, letta via Tauri da App.tsx) non
// è disponibile in modo sincrono qui: leggerla richiede un invoke asincrono
// verso il backend. init() parte quindi con resolveSystemLocale() come
// lingua iniziale (corretto nel caso comune: il default di Config.locale è
// "system" anche lato Rust) — se l'utente ha una preferenza esplicita
// diversa, App.tsx la applica con applyLocale() nello stesso effetto che
// già applica il tema, non appena Config è tornato dal backend (stesso
// principio di applyTheme in src/lib/theme.ts).

import i18next from "i18next";
import { initReactI18next } from "react-i18next";

import en from "./en";
import it from "./it";
import { resolveSystemLocale } from "./resolveSystemLocale";
import type { Locale } from "../lib/types";

void i18next.use(initReactI18next).init({
  resources: {
    it: { translation: it },
    en: { translation: en },
  },
  lng: resolveSystemLocale(),
  fallbackLng: "en",
  interpolation: {
    escapeValue: false,
  },
});

/** Applica una preferenza di lingua (da Config.locale) all'istanza i18next
 * corrente: "system" viene risolto a "it"/"en" via resolveSystemLocale(),
 * un valore esplicito si usa così com'è. La UI si aggiorna subito — ogni
 * componente che usa useTranslation() è sottoscritto all'evento
 * languageChanged di i18next e ri-renderizza da solo. */
export function applyLocale(locale: Locale): void {
  void i18next.changeLanguage(locale === "system" ? resolveSystemLocale() : locale);
}

export default i18next;
