// cycleTaskState/toggleTaskMarker richiedono un Editor Tiptap vero (DOM +
// ProseMirror view) — fuori dall'ambito di questi test, stesso limite già
// documentato per linkAutocomplete.ts/tagAutocomplete.ts/moveBlock.ts.
// TASK_PATTERN e nextTaskState sono invece pura logica, testate qui.

import { describe, expect, it } from "vitest";

import { nextTaskState, TASK_PATTERN } from "./taskActions";

describe("TASK_PATTERN", () => {
  it("matches an undone task marker", () => {
    expect(TASK_PATTERN.test("[ ] comprare il latte")).toBe(true);
  });

  it("matches a done task marker, lowercase or uppercase x", () => {
    expect(TASK_PATTERN.test("[x] fatto")).toBe(true);
    expect(TASK_PATTERN.test("[X] fatto")).toBe(true);
  });

  it("does not match plain text", () => {
    expect(TASK_PATTERN.test("solo testo")).toBe(false);
  });

  it("only matches at the start of the string", () => {
    expect(TASK_PATTERN.test("nota: [ ] non è un task qui")).toBe(false);
  });
});

describe("nextTaskState", () => {
  it("normale -> [ ] (prima pressione)", () => {
    expect(nextTaskState("scrivere il diario")).toEqual({
      nextText: "[ ] scrivere il diario",
      cursorDelta: 4,
    });
  });

  it("[ ] -> [x] (seconda pressione)", () => {
    expect(nextTaskState("[ ] scrivere il diario")).toEqual({
      nextText: "[x] scrivere il diario",
      cursorDelta: 0,
    });
  });

  it("[x] -> normale (terza pressione, chiude il ciclo)", () => {
    expect(nextTaskState("[x] scrivere il diario")).toEqual({
      nextText: "scrivere il diario",
      cursorDelta: -4,
    });
  });

  it("[X] maiuscolo -> normale, come [x]", () => {
    expect(nextTaskState("[X] scrivere il diario")).toEqual({
      nextText: "scrivere il diario",
      cursorDelta: -4,
    });
  });

  it("un blocco vuoto diventa un task vuoto", () => {
    expect(nextTaskState("")).toEqual({ nextText: "[ ] ", cursorDelta: 4 });
  });
});
