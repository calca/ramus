import { beforeAll, describe, expect, it } from "vitest";

import i18n from "../i18n";
import { translateError } from "./errors";

// translateError() risolve le chiavi errors.* con l'istanza i18next
// corrente, la cui lingua iniziale dipende da navigator.language — reale,
// non stubbato, in questo file (stesso problema documentato in
// src/lib/shortcut.test.ts / src/lib/paletteActions.test.ts). Si forza la
// lingua invece di affidarsi a quella rilevata.
beforeAll(async () => {
  await i18n.changeLanguage("it");
});

describe("translateError", () => {
  it("translates a CoreError payload via i18next, interpolating params", () => {
    const err = { code: "invalid_date", params: { date: "non-una-data" } };
    expect(translateError(err)).toBe(
      "Data non valida, atteso formato YYYY-MM-DD: non-una-data",
    );
  });

  it("translates a zero-payload CoreError code", () => {
    const err = { code: "poisoned_config_lock", params: {} };
    expect(translateError(err)).toBe("Stato di configurazione corrotto");
  });

  it("falls back to String(err) for a non-CoreError value", () => {
    expect(translateError(new Error("boom"))).toBe(String(new Error("boom")));
    expect(translateError("plain string")).toBe("plain string");
  });
});
