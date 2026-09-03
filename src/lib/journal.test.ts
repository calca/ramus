// formatJournalHeader dipende da "oggi" (etichette relative "Oggi"/"Ieri"/
// "N giorni fa") — orologio di sistema fissato con vi.setSystemTime per
// non far dipendere l'esito del test dal giorno reale in cui gira.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  formatIsoDate,
  formatJournalHeader,
  formatPrettyDate,
  journalDateFromPath,
  parseTypedDate,
} from "./journal";

describe("formatIsoDate", () => {
  it("formats a local date as YYYY-MM-DD, zero-padded", () => {
    expect(formatIsoDate(new Date(2026, 0, 5))).toBe("2026-01-05");
    expect(formatIsoDate(new Date(2026, 8, 3))).toBe("2026-09-03");
  });
});

describe("journalDateFromPath", () => {
  it("extracts the ISO date from a journal path", () => {
    expect(journalDateFromPath("journals/2026-09-02.md")).toBe("2026-09-02");
  });

  it("returns an empty string for a non-journal path", () => {
    expect(journalDateFromPath("pages/nota.md")).toBe("");
  });
});

describe("parseTypedDate", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 8, 3));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("recognizes ISO dates", () => {
    expect(parseTypedDate("2026-08-15")).toBe("2026-08-15");
  });

  it("recognizes Italian dates with / or -", () => {
    expect(parseTypedDate("15/08/2026")).toBe("2026-08-15");
    expect(parseTypedDate("15-08-2026")).toBe("2026-08-15");
  });

  it("accepts single-digit day/month in both formats", () => {
    expect(parseTypedDate("2026-8-5")).toBe("2026-08-05");
    expect(parseTypedDate("5/8/2026")).toBe("2026-08-05");
  });

  it("rejects a non-existent date instead of rolling it over", () => {
    expect(parseTypedDate("2026-04-31")).toBeNull();
    expect(parseTypedDate("31/04/2026")).toBeNull();
  });

  it("rejects a future date", () => {
    expect(parseTypedDate("2099-01-01")).toBeNull();
  });

  it("accepts today", () => {
    expect(parseTypedDate("2026-09-03")).toBe("2026-09-03");
  });

  it("returns null for text that isn't a date", () => {
    expect(parseTypedDate("ciao")).toBeNull();
    expect(parseTypedDate("")).toBeNull();
  });
});

describe("formatPrettyDate", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 8, 3));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("omits the year for the current year", () => {
    expect(formatPrettyDate("2026-08-15")).toBe("15 agosto");
  });

  it("includes the year for a different year", () => {
    expect(formatPrettyDate("2025-08-15")).toBe("15 agosto 2025");
  });
});

describe("formatJournalHeader", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 8, 3));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("labels today as Oggi", () => {
    expect(formatJournalHeader("2026-09-03")).toBe("Oggi 3 settembre");
  });

  it("labels yesterday as Ieri", () => {
    expect(formatJournalHeader("2026-09-02")).toBe("Ieri 2 settembre");
  });

  it("labels 2-6 days ago as relative", () => {
    expect(formatJournalHeader("2026-08-30")).toBe("4 giorni fa 30 agosto");
  });

  it("labels a week or more ago with the weekday", () => {
    // 2026-08-20 è più di 7 giorni prima del 2026-09-03: niente etichetta
    // relativa, solo il giorno della settimana.
    expect(formatJournalHeader("2026-08-20")).toMatch(/^[A-ZÀ-Ü][a-zà-ü]+ 20 agosto$/);
  });
});
