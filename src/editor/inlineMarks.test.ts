import { describe, expect, it } from "vitest";

import { escapeInlineText, parseInlineMarks } from "./inlineMarks";

describe("parseInlineMarks", () => {
  it("returns a single unmarked run for plain text", () => {
    expect(parseInlineMarks("ciao mondo")).toEqual([{ text: "ciao mondo", marks: [] }]);
  });

  it("recognizes bold", () => {
    expect(parseInlineMarks("**ciao**")).toEqual([{ text: "ciao", marks: ["bold"] }]);
  });

  it("recognizes italic", () => {
    expect(parseInlineMarks("*ciao*")).toEqual([{ text: "ciao", marks: ["italic"] }]);
  });

  it("recognizes bold+italic combined", () => {
    expect(parseInlineMarks("***ciao***")).toEqual([{ text: "ciao", marks: ["bold", "italic"] }]);
  });

  it("splits plain/marked/plain into separate runs", () => {
    expect(parseInlineMarks("prima **grassetto** dopo")).toEqual([
      { text: "prima ", marks: [] },
      { text: "grassetto", marks: ["bold"] },
      { text: " dopo", marks: [] },
    ]);
  });

  it("handles two separate marked spans in the same block", () => {
    expect(parseInlineMarks("*uno* e *due*")).toEqual([
      { text: "uno", marks: ["italic"] },
      { text: " e ", marks: [] },
      { text: "due", marks: ["italic"] },
    ]);
  });

  it("un-escapes a literal asterisk inside plain text", () => {
    expect(parseInlineMarks("2\\*3=6")).toEqual([{ text: "2*3=6", marks: [] }]);
  });

  it("un-escapes a literal backslash inside plain text", () => {
    expect(parseInlineMarks("C:\\\\Users")).toEqual([{ text: "C:\\Users", marks: [] }]);
  });

  it("leaves an unterminated delimiter as literal text", () => {
    expect(parseInlineMarks("un * solo asterisco")).toEqual([
      { text: "un * solo asterisco", marks: [] },
    ]);
  });

  it("returns no runs for empty content", () => {
    expect(parseInlineMarks("")).toEqual([]);
  });
});

describe("escapeInlineText", () => {
  it("leaves plain text untouched", () => {
    expect(escapeInlineText("ciao mondo")).toBe("ciao mondo");
  });

  it("escapes a literal asterisk", () => {
    expect(escapeInlineText("2*3=6")).toBe("2\\*3=6");
  });

  it("escapes a literal backslash", () => {
    expect(escapeInlineText("C:\\Users")).toBe("C:\\\\Users");
  });
});

describe("escapeInlineText + parseInlineMarks round-trip", () => {
  const cases = ["ciao mondo", "2*3=6", "C:\\Users", "path\\*name", "***", "**", "*"];

  for (const text of cases) {
    it(`round-trips ${JSON.stringify(text)}`, () => {
      const escaped = escapeInlineText(text);
      const runs = parseInlineMarks(escaped);
      expect(runs.map((run) => run.text).join("")).toBe(text);
      expect(runs.every((run) => run.marks.length === 0)).toBe(true);
    });
  }
});
