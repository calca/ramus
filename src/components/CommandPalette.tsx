import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";

import { listPages, search } from "../lib/commands";
import { formatJournalHeader, journalDateFromPath } from "../lib/journal";
import type { PaletteAction } from "../lib/paletteActions";
import type { PageSummary, SearchHit } from "../lib/types";
import { Modal } from "./Modal";

export type PaletteItem =
  | { kind: "action"; action: PaletteAction }
  | { kind: "recent"; title: string }
  | { kind: "hit"; hit: SearchHit }
  | { kind: "create"; title: string };

interface CommandPaletteProps {
  actions: PaletteAction[];
  recentPages: string[];
  onClose: () => void;
  onSelect: (item: PaletteItem) => void;
}

const DEBOUNCE_MS = 250;

const SECTION_LABELS: Record<PaletteItem["kind"], string> = {
  action: "Azioni",
  recent: "Recenti",
  hit: "Risultati",
  create: "Crea",
};

/** Evoluzione di SearchPanel (M2): ricerca full-text invariata, più
 * azioni dell'app, pagine aperte di recente e creazione pagine — vedi
 * specs/M4/2026-09-02-command-palette.TODO.md. */
export function CommandPalette({ actions, recentPages, onClose, onSelect }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [pages, setPages] = useState<PageSummary[]>([]);
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    inputRef.current?.focus();
    void listPages().then(setPages);
  }, []);

  useEffect(() => {
    setSelected(0);
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
    }
    const trimmed = query.trim();
    if (!trimmed) {
      setHits([]);
      return;
    }
    timerRef.current = setTimeout(() => {
      void search(trimmed).then(setHits);
    }, DEBOUNCE_MS);
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
      }
    };
  }, [query]);

  const items = useMemo<PaletteItem[]>(() => {
    const trimmed = query.trim();
    if (!trimmed) {
      return [
        ...recentPages.map((title): PaletteItem => ({ kind: "recent", title })),
        ...actions.map((action): PaletteItem => ({ kind: "action", action })),
      ];
    }
    const lower = trimmed.toLowerCase();
    const matchingActions = actions
      .filter((action) => action.label.toLowerCase().includes(lower))
      .map((action): PaletteItem => ({ kind: "action", action }));
    const hitItems = hits.map((hit): PaletteItem => ({ kind: "hit", hit }));
    const alreadyExists = pages.some((page) => page.title.toLowerCase() === lower);
    const createItem: PaletteItem[] = alreadyExists ? [] : [{ kind: "create", title: trimmed }];
    return [...matchingActions, ...hitItems, ...createItem];
  }, [query, actions, recentPages, hits, pages]);

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>) => {
    if (items.length === 0) {
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelected((index) => (index + 1) % items.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelected((index) => (index - 1 + items.length) % items.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const item = items[selected];
      if (item) {
        onSelect(item);
      }
    }
  };

  let lastKind: PaletteItem["kind"] | null = null;

  return (
    <Modal onClose={onClose} ariaLabel="Comandi">
      <input
        ref={inputRef}
        type="text"
        className="palette-input"
        placeholder="Cerca, crea o esegui un comando…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        onKeyDown={handleKeyDown}
      />
      {items.length > 0 && (
        <ul className="palette-results">
          {items.map((item, index) => {
            const showHeader = item.kind !== lastKind;
            lastKind = item.kind;
            const key =
              item.kind === "action"
                ? `action-${item.action.id}`
                : item.kind === "hit"
                  ? `hit-${item.hit.path}-${index}`
                  : `${item.kind}-${item.title}`;
            return (
              <li key={key}>
                {showHeader && <p className="palette-section-label">{SECTION_LABELS[item.kind]}</p>}
                <button
                  type="button"
                  className={index === selected ? "palette-item is-selected" : "palette-item"}
                  onMouseEnter={() => setSelected(index)}
                  onClick={() => onSelect(item)}
                >
                  {item.kind === "action" && (
                    <span className="palette-item-title">{item.action.label}</span>
                  )}
                  {item.kind === "recent" && <span className="palette-item-title">{item.title}</span>}
                  {item.kind === "create" && (
                    <span className="palette-item-title">Crea «{item.title}»</span>
                  )}
                  {item.kind === "hit" && (
                    <>
                      <span className="palette-item-title">
                        {item.hit.kind === "journal"
                          ? formatJournalHeader(journalDateFromPath(item.hit.path))
                          : (item.hit.title ?? item.hit.path)}
                      </span>
                      {item.hit.snippet_html && (
                        <span
                          className="palette-item-snippet"
                          dangerouslySetInnerHTML={{ __html: item.hit.snippet_html }}
                        />
                      )}
                    </>
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </Modal>
  );
}
