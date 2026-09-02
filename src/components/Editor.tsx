import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { EditorContent, useEditor } from "@tiptap/react";

import { writePage } from "../lib/commands";
import type { Block, Page } from "../lib/types";
import { blocksToDoc } from "../editor/deserializer";
import { createExtensions } from "../editor/extensions";
import type { PMNode } from "../editor/pmNode";
import { docToBlocks } from "../editor/serializer";

const SAVE_DEBOUNCE_MS = 500;

export interface EditorHandle {
  /** Salva subito eventuali modifiche non ancora scritte su disco. */
  flush: () => Promise<void>;
}

interface EditorProps {
  page: Page;
  onDirtyChange: (dirty: boolean) => void;
}

export const Editor = forwardRef<EditorHandle, EditorProps>(function Editor(
  { page, onDirtyChange },
  ref,
) {
  const pendingBlocksRef = useRef<Block[] | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const save = async () => {
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    const blocks = pendingBlocksRef.current;
    if (blocks === null) {
      return;
    }
    pendingBlocksRef.current = null;
    await writePage(page.path, blocks);
    onDirtyChange(false);
  };

  const editor = useEditor(
    {
      extensions: createExtensions(),
      content: blocksToDoc(page.blocks),
      onUpdate: ({ editor: current }) => {
        pendingBlocksRef.current = docToBlocks(current.getJSON() as PMNode);
        onDirtyChange(true);
        if (timerRef.current !== null) {
          clearTimeout(timerRef.current);
        }
        timerRef.current = setTimeout(() => {
          void save();
        }, SAVE_DEBOUNCE_MS);
      },
    },
    [page.path],
  );

  useImperativeHandle(ref, () => ({ flush: save }), [page.path]);

  useEffect(() => {
    const onBlur = () => {
      void save();
    };
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  }, [page.path]);

  return <EditorContent className="ramus-editor" editor={editor} />;
});
