import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import type { MouseEvent } from "react";
import { EditorContent, useEditor } from "@tiptap/react";

import { writePage } from "../lib/commands";
import type { Block, Page } from "../lib/types";
import { blocksToDoc } from "../editor/deserializer";
import { createExtensions } from "../editor/extensions";
import type { PMNode } from "../editor/pmNode";
import { docToBlocks } from "../editor/serializer";
import { toggleTaskMarker } from "../editor/taskActions";

const SAVE_DEBOUNCE_MS = 500;

export interface EditorHandle {
  /** Salva subito eventuali modifiche non ancora scritte su disco. */
  flush: () => Promise<void>;
  /** Sposta il cursore dentro questo editor (navigazione fra giorni da
   * tastiera, M4). */
  focus: () => void;
}

interface EditorProps {
  page: Page;
  onDirtyChange: (dirty: boolean) => void;
  /** Click su un [[link]] (la decorazione .editor-link porta il testo del
   * link in data-title — nessun bisogno di ri-matchare la regex qui). */
  onLinkClick?: (title: string) => void;
}

export const Editor = forwardRef<EditorHandle, EditorProps>(function Editor(
  { page, onDirtyChange, onLinkClick },
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

  useImperativeHandle(
    ref,
    () => ({ flush: save, focus: () => editor?.commands.focus() }),
    [page.path, editor],
  );

  useEffect(() => {
    const onBlur = () => {
      void save();
    };
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  }, [page.path]);

  const handleClick = (event: MouseEvent<HTMLDivElement>) => {
    const target = event.target as HTMLElement;
    const taskMarker = target.closest(".editor-task-marker");
    if (taskMarker && editor) {
      toggleTaskMarker(editor, taskMarker);
      return;
    }
    const link = target.closest(".editor-link");
    const title = link?.getAttribute("data-title");
    if (title) {
      onLinkClick?.(title);
    }
  };

  return <EditorContent className="ramus-editor" editor={editor} onClick={handleClick} />;
});
