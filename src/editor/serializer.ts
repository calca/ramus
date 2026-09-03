// JSON documento ProseMirror -> Block[]. Funzione scritta a mano (niente
// estensioni markdown di terze parti): ogni text node del paragrafo viene
// riportato a sintassi markdown in base ai suoi marks (bold/italic, vedi
// editor/inlineMarks.ts) e concatenato — il resto del blocco (link `[[..]]`,
// tag `#..`) resta testo semplice, invariato.

import { escapeInlineText } from "./inlineMarks";
import type { Block } from "../lib/types";
import type { PMNode } from "./pmNode";

function wrapForMarks(text: string, markTypes: string[]): string {
  const bold = markTypes.includes("bold");
  const italic = markTypes.includes("italic");
  if (bold && italic) {
    return `***${text}***`;
  }
  if (bold) {
    return `**${text}**`;
  }
  if (italic) {
    return `*${text}*`;
  }
  return text;
}

function textNodeToMarkdown(node: PMNode & { text: string }): string {
  const escaped = escapeInlineText(node.text);
  const markTypes = (node.marks ?? []).map((mark) => mark.type);
  return wrapForMarks(escaped, markTypes);
}

function paragraphText(paragraph: PMNode): string {
  if (!paragraph.content) {
    return "";
  }
  return paragraph.content
    .filter((node): node is PMNode & { text: string } => node.type === "text" && typeof node.text === "string")
    .map(textNodeToMarkdown)
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
