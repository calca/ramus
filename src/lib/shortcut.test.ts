// IS_MAC (in shortcut.ts) è calcolata una volta sola al caricamento del
// modulo, leggendo `navigator.platform` — Node ha un `navigator` globale
// che riflette il sistema operativo REALE della macchina che esegue i
// test, diverso fra questa macchina (macOS) e il runner Linux della CI
// (specs/release/2026-09-03-ci.TODO.md). Ogni test qui sotto sceglie la
// piattaforma esplicitamente (`vi.stubGlobal` + `vi.resetModules()` +
// import dinamico) invece di affidarsi a quella reale — altrimenti lo
// stesso test avrebbe esito diverso in locale e in CI.

import { afterEach, describe, expect, it, vi } from "vitest";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetModules();
});

function stubPlatform(platform: "mac" | "other") {
  vi.stubGlobal(
    "navigator",
    platform === "mac"
      ? { platform: "MacIntel", userAgent: "Macintosh" }
      : { platform: "Win32", userAgent: "Windows" },
  );
}

function keyEvent(overrides: Partial<KeyboardEvent> & { key: string }): KeyboardEvent {
  return {
    shiftKey: false,
    altKey: false,
    metaKey: false,
    ctrlKey: false,
    ...overrides,
  } as KeyboardEvent;
}

describe("getShortcut", () => {
  it("returns the configured override when present", async () => {
    const { getShortcut } = await import("./shortcut");
    expect(getShortcut({ cheatsheet: "Mod+Shift+K" }, "cheatsheet")).toBe("Mod+Shift+K");
  });

  it("falls back to the registry default when missing from config", async () => {
    const { getShortcut } = await import("./shortcut");
    expect(getShortcut({}, "cheatsheet")).toBe("Mod+/");
  });

  it("returns an empty string for an unregistered action id", async () => {
    const { getShortcut } = await import("./shortcut");
    expect(getShortcut({}, "not-a-real-action")).toBe("");
  });
});

describe("normalizeShortcut su macOS (modificatore primario: Cmd)", () => {
  it("normalizes Cmd+K", async () => {
    stubPlatform("mac");
    const { normalizeShortcut } = await import("./shortcut");
    expect(normalizeShortcut(keyEvent({ key: "k", metaKey: true }))).toBe("Mod+K");
  });

  it("includes Shift/Alt when held", async () => {
    stubPlatform("mac");
    const { normalizeShortcut } = await import("./shortcut");
    expect(
      normalizeShortcut(keyEvent({ key: "k", metaKey: true, shiftKey: true, altKey: true })),
    ).toBe("Mod+Shift+Alt+K");
  });

  it("returns null without the primary modifier", async () => {
    stubPlatform("mac");
    const { normalizeShortcut } = await import("./shortcut");
    expect(normalizeShortcut(keyEvent({ key: "k" }))).toBeNull();
  });

  it("returns null for a bare modifier key press", async () => {
    stubPlatform("mac");
    const { normalizeShortcut } = await import("./shortcut");
    expect(normalizeShortcut(keyEvent({ key: "Meta", metaKey: true }))).toBeNull();
  });

  it("ignora Ctrl da solo (non è il modificatore primario su Mac)", async () => {
    stubPlatform("mac");
    const { normalizeShortcut } = await import("./shortcut");
    expect(normalizeShortcut(keyEvent({ key: "k", ctrlKey: true }))).toBeNull();
  });
});

describe("normalizeShortcut fuori macOS (modificatore primario: Ctrl)", () => {
  it("normalizes Ctrl+K", async () => {
    stubPlatform("other");
    const { normalizeShortcut } = await import("./shortcut");
    expect(normalizeShortcut(keyEvent({ key: "k", ctrlKey: true }))).toBe("Mod+K");
  });

  it("ignora Cmd/Meta da solo (non è il modificatore primario fuori Mac)", async () => {
    stubPlatform("other");
    const { normalizeShortcut } = await import("./shortcut");
    expect(normalizeShortcut(keyEvent({ key: "k", metaKey: true }))).toBeNull();
  });
});

describe("matchesShortcut", () => {
  it("matches on macOS with Cmd held", async () => {
    stubPlatform("mac");
    const { matchesShortcut } = await import("./shortcut");
    expect(matchesShortcut(keyEvent({ key: "k", metaKey: true }), "Mod+K")).toBe(true);
  });

  it("does not match when Shift is required but not held", async () => {
    stubPlatform("mac");
    const { matchesShortcut } = await import("./shortcut");
    expect(matchesShortcut(keyEvent({ key: "k", metaKey: true }), "Mod+Shift+K")).toBe(false);
  });

  it("does not match a different key", async () => {
    stubPlatform("mac");
    const { matchesShortcut } = await import("./shortcut");
    expect(matchesShortcut(keyEvent({ key: "j", metaKey: true }), "Mod+K")).toBe(false);
  });

  it("does not match without the primary modifier", async () => {
    stubPlatform("mac");
    const { matchesShortcut } = await import("./shortcut");
    expect(matchesShortcut(keyEvent({ key: "k" }), "Mod+K")).toBe(false);
  });

  it("matches special (non single-char) keys like ArrowUp", async () => {
    stubPlatform("mac");
    const { matchesShortcut } = await import("./shortcut");
    expect(matchesShortcut(keyEvent({ key: "ArrowUp", metaKey: true }), "Mod+ArrowUp")).toBe(
      true,
    );
  });
});

describe("formatShortcut", () => {
  it("uses Mac symbols on macOS", async () => {
    stubPlatform("mac");
    const { formatShortcut } = await import("./shortcut");
    expect(formatShortcut("Mod+K")).toBe("⌘K");
    expect(formatShortcut("Mod+Shift+F")).toBe("⌘⇧F");
    expect(formatShortcut("Mod+ArrowUp")).toBe("⌘↑");
  });

  it("uses Ctrl+ labels off macOS", async () => {
    stubPlatform("other");
    const { formatShortcut } = await import("./shortcut");
    expect(formatShortcut("Mod+K")).toBe("Ctrl+K");
    expect(formatShortcut("Mod+Shift+F")).toBe("Ctrl+Shift+F");
    expect(formatShortcut("Mod+ArrowUp")).toBe("Ctrl+↑");
  });
});
