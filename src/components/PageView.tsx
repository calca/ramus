import type { EditorHandle } from "./Editor";
import { Editor } from "./Editor";
import type { Page } from "../lib/types";

interface PageViewProps {
  page: Page;
  onDirtyChange: (dirty: boolean) => void;
  onLinkClick: (title: string) => void;
  onBack: () => void;
  registerEditorHandle: (handle: EditorHandle | null) => void;
}

export function PageView({ page, onDirtyChange, onLinkClick, onBack, registerEditorHandle }: PageViewProps) {
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
      </div>
    </div>
  );
}
