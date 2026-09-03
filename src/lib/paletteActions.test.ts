import { describe, expect, it, vi } from "vitest";

import { buildActions } from "./paletteActions";
import type { PaletteActionContext } from "./paletteActions";

function baseContext(overrides: Partial<PaletteActionContext> = {}): PaletteActionContext {
  return {
    viewKind: "journal",
    isCompact: false,
    onToday: vi.fn(),
    onReturnToJournal: vi.fn(),
    onToggleCompact: vi.fn(),
    onOpenSettings: vi.fn(),
    onShowAbout: vi.fn(),
    onShowCheatsheet: vi.fn(),
    ...overrides,
  };
}

describe("buildActions", () => {
  it("shows 'Vai a oggi' when viewing the journal", () => {
    const actions = buildActions(baseContext({ viewKind: "journal" }));
    expect(actions.map((a) => a.id)).toContain("today");
    expect(actions.map((a) => a.id)).not.toContain("return-journal");
  });

  it("shows 'Torna al journal' when viewing a page", () => {
    const actions = buildActions(baseContext({ viewKind: "page" }));
    expect(actions.map((a) => a.id)).toContain("return-journal");
    expect(actions.map((a) => a.id)).not.toContain("today");
  });

  it("labels the compact toggle based on current state", () => {
    const expanded = buildActions(baseContext({ isCompact: false }));
    expect(expanded.find((a) => a.id === "toggle-compact")?.label).toBe("Comprimi finestra");

    const compact = buildActions(baseContext({ isCompact: true }));
    expect(compact.find((a) => a.id === "toggle-compact")?.label).toBe("Espandi finestra");
  });

  it("always includes settings, about and cheatsheet", () => {
    const ids = buildActions(baseContext()).map((a) => a.id);
    expect(ids).toEqual(expect.arrayContaining(["settings", "about", "cheatsheet"]));
  });

  it("wires each action's run callback to the matching context handler", () => {
    const onToday = vi.fn();
    const actions = buildActions(baseContext({ viewKind: "journal", onToday }));
    actions.find((a) => a.id === "today")?.run();
    expect(onToday).toHaveBeenCalledOnce();
  });

  it("returns exactly five actions (viewKind contributes exactly one)", () => {
    expect(buildActions(baseContext({ viewKind: "journal" }))).toHaveLength(5);
    expect(buildActions(baseContext({ viewKind: "page" }))).toHaveLength(5);
  });
});
