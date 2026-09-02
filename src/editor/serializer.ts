// JSON documento ProseMirror -> Block[]. Funzione scritta a mano (niente
// estensioni markdown di terze parti): il contenuto di ogni blocco è il
// testo semplice del suo paragrafo, senza marks (disattivati in
// extensions.ts), quindi non serve gestire la ricostruzione di sintassi
// markdown a partire dai marks.

import type { Block } from "../lib/types";
import type { PMNode } from "./pmNode";

function paragraphText(paragraph: PMNode): string {
  if (!paragraph.content) {
    return "";
  }
  return paragraph.content
    .filter((node): node is PMNode & { text: string } => node.type === "text" && typeof node.text === "string")
    .map((node) => node.text)
    .join("");
}

function listItemToBlock(item: PMNode): Block {
  const children = item.content ?? [];
  const paragraph = children.find((node) => node.type === "paragraph");
  const nestedList = children.find((node) => node.type === "bulletList");

  return {
    content: paragraph ? paragraphText(paragraph) : "",
    children: nestedList ? bulletListToBlocks(nestedList) : [],
  };
}

function bulletListToBlocks(bulletList: PMNode): Block[] {
  return (bulletList.content ?? [])
    .filter((node) => node.type === "listItem")
    .map(listItemToBlock);
}

/** Converte il documento dell'editor (`editor.getJSON()`) in `Block[]`. */
export function docToBlocks(doc: PMNode): Block[] {
  const bulletList = (doc.content ?? []).find((node) => node.type === "bulletList");
  return bulletList ? bulletListToBlocks(bulletList) : [];
}
