// Sottoinsieme minimo del JSON documento di ProseMirror usato da
// serializer/deserializer. Non è lo schema completo di Tiptap: solo i nodi
// che l'editor outliner produce davvero (doc, bulletList, listItem,
// paragraph, text).

export interface PMMark {
  type: string;
}

export interface PMNode {
  type: string;
  content?: PMNode[];
  text?: string;
  marks?: PMMark[];
}
