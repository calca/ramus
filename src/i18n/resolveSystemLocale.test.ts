// navigator in Node riflette il sistema operativo/locale REALE della
// macchina che esegue i test (navigator.language legge LANG/LC_ALL, diverso
// fra questa macchina e il runner CI) — stesso problema, stessa soluzione
// già documentata in src/lib/shortcut.test.ts: ogni test stuba `navigator`
// esplicitamente invece di affidarsi a quello reale, con `vi.resetModules()`
// + import dinamico perché resolveSystemLocale legge `navigator` ad ogni
// chiamata (non al load del modulo, ma lo stub va comunque isolato per
// coerenza con lo stesso pattern).

import { afterEach, describe, expect, it, vi } from "vitest";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetModules();
});

describe("resolveSystemLocale", () => {
  it("returns 'it' when navigator.language starts with 'it'", async () => {
    vi.stubGlobal("navigator", { language: "it-IT" });
    const { resolveSystemLocale } = await import("./resolveSystemLocale");
    expect(resolveSystemLocale()).toBe("it");
  });

  it("returns 'it' for a bare 'it' language tag", async () => {
    vi.stubGlobal("navigator", { language: "it" });
    const { resolveSystemLocale } = await import("./resolveSystemLocale");
    expect(resolveSystemLocale()).toBe("it");
  });

  it("is case-insensitive", async () => {
    vi.stubGlobal("navigator", { language: "IT-it" });
    const { resolveSystemLocale } = await import("./resolveSystemLocale");
    expect(resolveSystemLocale()).toBe("it");
  });

  it("returns 'en' for English", async () => {
    vi.stubGlobal("navigator", { language: "en-US" });
    const { resolveSystemLocale } = await import("./resolveSystemLocale");
    expect(resolveSystemLocale()).toBe("en");
  });

  it("returns 'en' as the universal fallback for any other language", async () => {
    vi.stubGlobal("navigator", { language: "fr-FR" });
    const { resolveSystemLocale } = await import("./resolveSystemLocale");
    expect(resolveSystemLocale()).toBe("en");

    vi.stubGlobal("navigator", { language: "de-DE" });
    vi.resetModules();
    const { resolveSystemLocale: resolveAgain } = await import("./resolveSystemLocale");
    expect(resolveAgain()).toBe("en");
  });
});
