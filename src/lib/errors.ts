// Traduce un errore ricevuto da un command Tauri. `CoreError` (lato Rust,
// crates/ramus-core/src/error.rs) serializza verso il frontend come
// `{code, params}` invece di una stringa già composta in italiano — vedi
// specs/M7/2026-09-04-i18n-errori.DONE.md. Questo è l'unico punto che
// traduce: ogni componente che cattura un errore passa da qui invece di
// fare `String(err)` direttamente, altrimenti l'utente vedrebbe sempre e
// solo l'italiano lato Rust indipendentemente dalla lingua scelta.

import i18next from "../i18n";

interface CoreErrorPayload {
  code: string;
  params: Record<string, string | number>;
}

function isCoreErrorPayload(err: unknown): err is CoreErrorPayload {
  return (
    typeof err === "object" &&
    err !== null &&
    "code" in err &&
    typeof (err as { code: unknown }).code === "string" &&
    "params" in err
  );
}

/** Traduce un errore di un command Tauri (`CoreError`) nella lingua attiva.
 * Un errore che non ha la forma `{code, params}` (JS nativo, errore interno
 * di Tauri) passa invariato da `String(err)` — non è un `CoreError`, non
 * c'è nulla da tradurre. */
export function translateError(err: unknown): string {
  if (isCoreErrorPayload(err)) {
    return i18next.t(`errors.${err.code}`, err.params);
  }
  return String(err);
}
