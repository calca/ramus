import { useEffect, useState } from "react";

import { BacklinksSection } from "./BacklinksSection";
import type { EditorHandle } from "./Editor";
import { Editor } from "./Editor";
import { findBacklinks } from "../lib/commands";
import type { Backlink, Page } from "../lib/types";

interface PageViewProps {
  page: Page;
  onDirtyChange: (dirty: boolean) => void;
  onLinkClick: (title: string) => void;
  onBack: () => void;
  registerEditorHandle: (handle: EditorHandle | null) => void;
}

/** Titolo da usare per cercare i backlink: quello del front-matter, o —
 * caso limite non raggiungibile dal flusso normale dell'app, `open_page`
 * scrive sempre un front-matter — lo slug derivato dal path stesso
 * (già uno slug valido per costruzione, a differenza del path intero). */
function backlinkTarget(page: Page): string {
  if (page.title) {
    return page.title;
  }
  return page.path.replace(/^pages\//, "").replace(/\.md$/, "");
}

export function PageView({ page, onDirtyChange, onLinkClick, onBack, registerEditorHandle }: PageViewProps) {
  const [backlinks, setBacklinks] = useState<Backlink[]>([]);

  useEffect(() => {
    let cancelled = false;
    void findBacklinks(backlinkTarget(page)).then((result) => {
      if (!cancelled) {
        setBacklinks(result);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [page.path, page.title]);

  return (
    <div className="page-view">
      <div className="page-view-content">
        <button type="button" className="page-view-back" onClick={onBack}>
          ← Journal
        </button>
        <h1 className="page-view-title">{page.title ?? page.path}</h1>
        <Editor
          ref={registerEditorHandle}
          page={page}
          onDirtyChange={onDirtyChange}
          onLinkClick={onLinkClick}
        />
        <BacklinksSection backlinks={backlinks} onSelect={onLinkClick} />
      </div>
    </div>
  );
}
