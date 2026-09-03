// Grassetto/corsivo <-> markdown, scritto a mano (CLAUDE.md: niente
// librerie markdown per Tiptap). Sintassi CommonMark/Obsidian-compatibile:
// **grassetto**, *corsivo*, ***entrambi***. Volutamente senza emphasis
// nidificato (es. "**bold *italic* bold**" non viene riconosciuto come
// due mark separati): un delimitatore aperto si chiude alla prima
// occorrenza di un delimitatore della stessa lunghezza, senza scansionare
// ricorsivamente il contenuto — vedi
// specs/refinement/2026-09-03-testo-grassetto-corsivo.DONE.md.

export type InlineMark = "bold" | "italic";

export interface InlineRun {
  text: string;
  marks: InlineMark[];
}

const DELIMITER_MARKS: Record<number, InlineMark[]> = {
  1: ["italic"],
  2: ["bold"],
  3: ["bold", "italic"],
};

/** Sfugge `\` e `*` letterali prima di scriverli su disco: senza, un
 * testo semplice come "2*3=6" verrebbe riletto come corsivo al prossimo
 * caricamento. Unico modo per garantire il round-trip (CLAUDE.md, regola
 * 5) anche per testo che contiene questi due caratteri senza intenzione
 * di formattazione. */
export function escapeInlineText(text: string): string {
  let out = "";
  for (const ch of text) {
    out += ch === "\\" || ch === "*" ? "\\" + ch : ch;
  }
  return out;
}

/** Converte il contenuto grezzo di un blocco (già salvato su disco, o
 * digitato in un altro editor markdown) in run di testo con i marks
 * applicati — l'inverso a mano di escapeInlineText + del wrapping fatto
 * dal serializer. */
export function parseInlineMarks(source: string): InlineRun[] {
  const runs: InlineRun[] = [];
  let buffer = "";

  const flush = () => {
    if (buffer) {
      runs.push({ text: buffer, marks: [] });
      buffer = "";
    }
  };

  let i = 0;
  while (i < source.length) {
    const ch = source[i];

    if (ch === "\\" && (source[i + 1] === "\\" || source[i + 1] === "*")) {
      buffer += source[i + 1];
      i += 2;
      continue;
    }

    if (ch === "*") {
      let runLength = 0;
      while (source[i + runLength] === "*") {
        runLength++;
      }
      const delimiterLength = Math.min(runLength, 3);
      const closeIndex = findClosingDelimiter(source, i + delimiterLength, delimiterLength);
      if (closeIndex !== -1) {
        flush();
        const inner = source.slice(i + delimiterLength, closeIndex);
        runs.push({ text: unescapeInline(inner), marks: DELIMITER_MARKS[delimiterLength] });
        i = closeIndex + delimiterLength;
        continue;
      }
      // Nessun delimitatore di chiusura trovato: gli asterischi restano
      // testo semplice invece di un mark aperto e mai chiuso.
      buffer += "*".repeat(delimiterLength);
      i += delimiterLength;
      continue;
    }

    buffer += ch;
    i++;
  }

  flush();
  return runs;
}

function findClosingDelimiter(source: string, from: number, length: number): number {
  let i = from;
  while (i < source.length) {
    if (source[i] === "\\" && (source[i + 1] === "\\" || source[i + 1] === "*")) {
      i += 2;
      continue;
    }
    if (source[i] === "*") {
      let runLength = 0;
      while (source[i + runLength] === "*") {
        runLength++;
      }
      if (runLength >= length) {
        return i;
      }
      i += runLength;
      continue;
    }
    i++;
  }
  return -1;
}

function unescapeInline(text: string): string {
  let out = "";
  let i = 0;
  while (i < text.length) {
    if (text[i] === "\\" && (text[i + 1] === "\\" || text[i + 1] === "*")) {
      out += text[i + 1];
      i += 2;
      continue;
    }
    out += text[i];
    i++;
  }
  return out;
}
