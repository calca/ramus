// Block[] -> JSON documento ProseMirror, per idratare l'editor a partire da
// una Page caricata via read_page/open_today. Inverso di serializer.ts: il
// contenuto grezzo di ogni blocco viene ritokenizzato in run di testo con i
// marks corrispondenti (editor/inlineMarks.ts).

import { parseInlineMarks } from "./inlineMarks";
import type { Block } from "../lib/types";
import type { PMNode } from "./pmNode";

function contentToTextNodes(content: string): PMNode[] {
  return parseInlineMarks(content).map((run) => ({
    type: "text",
    text: run.text,
    ...(run.marks.length > 0 ? { marks: run.marks.map((type) => ({ type })) } : {}),
  }));
}

function blockToListItem(block: Block): PMNode {
  const textNodes = contentToTextNodes(block.content);
  const paragraph: PMNode = textNodes.length > 0 ? { type: "paragraph", content: textNodes } : { type: "paragraph" };

  const content: PMNode[] = [paragraph];
  if (block.children.length > 0) {
    content.push(blocksToBulletList(block.children));
  }

  return { type: "listItem", content };
}

function blocksToBulletList(blocks: Block[]): PMNode {
  return { type: "bulletList", content: blocks.map(blockToListItem) };
}

/** Costruisce il documento ProseMirror da caricare in `editor.commands.setContent`. */
export function blocksToDoc(blocks: Block[]): PMNode {
  // Un doc non può contenere una bulletList vuota: una pagina senza blocchi
  // (journal appena creato prima della prima scrittura) diventa un unico
  // blocco vuoto, coerente con Vault::open_today lato core.
  const nonEmpty = blocks.length > 0 ? blocks : [{ content: "", children: [] }];
  return { type: "doc", content: [blocksToBulletList(nonEmpty)] };
}
