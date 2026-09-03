// Round-trip Block[] -> doc -> Block[] con marks (CLAUDE.md, regola 5:
// "parse(render(page)) == page", richiesto per ogni modifica al
// parser/serializer). Il verso opposto (doc -> Block[] -> doc) non è
// testato: l'editor non produce mai un JSON ProseMirror arbitrario, solo
// ciò che blocksToDoc genera a sua volta — coperto dagli stessi casi.

import { describe, expect, it } from "vitest";

import { docToBlocks } from "./serializer";
import { blocksToDoc } from "./deserializer";
import type { Block } from "../lib/types";

function roundTrip(blocks: Block[]): Block[] {
  return docToBlocks(blocksToDoc(blocks));
}

describe("Block[] -> doc -> Block[] round-trip", () => {
  it("preserves plain text", () => {
    const blocks: Block[] = [{ content: "ciao mondo", children: [] }];
    expect(roundTrip(blocks)).toEqual(blocks);
  });

  it("preserves bold", () => {
    const blocks: Block[] = [{ content: "**grassetto**", children: [] }];
    expect(roundTrip(blocks)).toEqual(blocks);
  });

  it("preserves italic", () => {
    const blocks: Block[] = [{ content: "*corsivo*", children: [] }];
    expect(roundTrip(blocks)).toEqual(blocks);
  });

  it("preserves bold+italic combined", () => {
    const blocks: Block[] = [{ content: "***entrambi***", children: [] }];
    expect(roundTrip(blocks)).toEqual(blocks);
  });

  it("preserves plain/bold/plain in the same block", () => {
    const blocks: Block[] = [{ content: "prima **grassetto** dopo", children: [] }];
    expect(roundTrip(blocks)).toEqual(blocks);
  });

  it("preserves a literal (già sfuggito) asterisco in testo semplice", () => {
    // Block.content è sempre sorgente markdown, non testo visualizzato: un
    // asterisco letterale ci arriva già sfuggito da escapeInlineText (lo
    // stesso giro che farebbe l'editor digitandolo), non grezzo — un "2*3=6"
    // non sfuggito verrebbe letto come apertura di un corsivo mai chiuso,
    // comportamento corretto e non un bug (vedi
    // specs/refinement/2026-09-03-testo-grassetto-corsivo.DONE.md).
    const blocks: Block[] = [{ content: "2\\*3=6, non formattato", children: [] }];
    expect(roundTrip(blocks)).toEqual(blocks);
  });

  it("preserves nested children alongside marked text", () => {
    const blocks: Block[] = [
      {
        content: "**genitore**",
        children: [
          { content: "*figlio*", children: [] },
          { content: "figlio semplice", children: [] },
        ],
      },
    ];
    expect(roundTrip(blocks)).toEqual(blocks);
  });

  it("preserves an empty block", () => {
    const blocks: Block[] = [{ content: "", children: [] }];
    expect(roundTrip(blocks)).toEqual(blocks);
  });

  it("preserves links and tags untouched (nessun mark coinvolto)", () => {
    const blocks: Block[] = [{ content: "vedi [[altra pagina]] e #tag", children: [] }];
    expect(roundTrip(blocks)).toEqual(blocks);
  });
});
