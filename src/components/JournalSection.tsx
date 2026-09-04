import { useCallback } from "react";
import { useTranslation } from "react-i18next";

import { formatJournalHeader, journalDateFromPath } from "../lib/journal";
import type { Page } from "../lib/types";
import type { EditorHandle } from "./Editor";
import { Editor } from "./Editor";

interface JournalSectionProps {
  page: Page;
  isToday: boolean;
  externalChangeWarning: boolean;
  onDirtyChange: (path: string, dirty: boolean) => void;
  onLinkClick: (title: string) => void;
  registerElement: (path: string, element: HTMLElement | null) => void;
  registerEditorHandle: (path: string, handle: EditorHandle | null) => void;
}

export function JournalSection({
  page,
  isToday,
  externalChangeWarning,
  onDirtyChange,
  onLinkClick,
  registerElement,
  registerEditorHandle,
}: JournalSectionProps) {
  const { t } = useTranslation();
  const setElement = useCallback(
    (element: HTMLElement | null) => registerElement(page.path, element),
    [page.path, registerElement],
  );
  const setEditorHandle = useCallback(
    (handle: EditorHandle | null) => registerEditorHandle(page.path, handle),
    [page.path, registerEditorHandle],
  );
  const handleDirtyChange = useCallback(
    (dirty: boolean) => onDirtyChange(page.path, dirty),
    [page.path, onDirtyChange],
  );

  return (
    <section className="journal-section" ref={setElement} data-path={page.path}>
      <h2 className={isToday ? "journal-section-date journal-section-date-today" : "journal-section-date"}>
        {formatJournalHeader(journalDateFromPath(page.path))}
      </h2>
      {externalChangeWarning && (
        <div className="banner banner-warning">{t("journal.externalChangeWarning")}</div>
      )}
      <Editor
        ref={setEditorHandle}
        page={page}
        onDirtyChange={handleDirtyChange}
        onLinkClick={onLinkClick}
      />
    </section>
  );
}
