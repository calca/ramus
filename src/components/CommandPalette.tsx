import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent, ReactNode } from "react";

import { listPages, search } from "../lib/commands";
import { formatJournalHeader, formatPrettyDate, journalDateFromPath, parseTypedDate } from "../lib/journal";
import type { PaletteAction } from "../lib/paletteActions";
import { formatShortcut, getShortcut } from "../lib/shortcut";
import type { PageSummary, SearchHit } from "../lib/types";
import { Modal } from "./Modal";

export type PaletteItem =
  | { kind: "action"; action: PaletteAction }
  | { kind: "recent"; title: string }
  | { kind: "hit"; hit: SearchHit }
  | { kind: "create"; title: string }
  | { kind: "date"; iso: string };

interface CommandPaletteProps {
  actions: PaletteAction[];
  recentPages: string[];
  shortcuts: Record<string, string>;
  onClose: () => void;
  onSelect: (item: PaletteItem) => void;
}

/** Evidenzia la prima occorrenza di `query` in `text` con `<mark>` — solo
 * per etichette filtrate per sottostringa lato client (azioni, recenti),
 * non per i risultati di ricerca full-text (già evidenziati dal backend,
 * `snippet_html`). */
function highlightMatch(text: string, query: string): ReactNode {
  if (!query) {
    return text;
  }
  const index = text.toLowerCase().indexOf(query.toLowerCase());
  if (index === -1) {
    return text;
  }
  return (
    <>
      {text.slice(0, index)}
      <mark className="palette-match">{text.slice(index, index + query.length)}</mark>
      {text.slice(index + query.length)}
    </>
  );
}

const DEBOUNCE_MS = 250;

const SECTION_LABELS: Record<PaletteItem["kind"], string> = {
  action: "Azioni",
  recent: "Recenti",
  hit: "Risultati",
  create: "Crea",
  date: "Data",
};

/** Evoluzione di SearchPanel (M2): ricerca full-text invariata, più
 * azioni dell'app, pagine aperte di recente e creazione pagine — vedi
 * specs/M4/2026-09-02-command-palette.TODO.md. */
export function CommandPalette({
  actions,
  recentPages,
  shortcuts,
  onClose,
  onSelect,
}: CommandPaletteProps) {
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

  const trimmedQuery = query.trim();

  const items = useMemo<PaletteItem[]>(() => {
    const trimmed = query.trim();
    if (!trimmed) {
      return [
        ...recentPages.map((title): PaletteItem => ({ kind: "recent", title })),
        ...actions.map((action): PaletteItem => ({ kind: "action", action })),
      ];
    }
    const lower = trimmed.toLowerCase();
    const iso = parseTypedDate(trimmed);
    // In cima, prima di azioni e risultati: se l'intero input è una data
    // valida è quasi certamente quello che si vuole, più affidabile di un
    // match fuzzy sul testo.
    const dateItem: PaletteItem[] = iso ? [{ kind: "date", iso }] : [];
    const matchingActions = actions
      .filter((action) => action.label.toLowerCase().includes(lower))
      .map((action): PaletteItem => ({ kind: "action", action }));
    const hitItems = hits.map((hit): PaletteItem => ({ kind: "hit", hit }));
    const alreadyExists = pages.some((page) => page.title.toLowerCase() === lower);
    const createItem: PaletteItem[] = alreadyExists ? [] : [{ kind: "create", title: trimmed }];
    return [...dateItem, ...matchingActions, ...hitItems, ...createItem];
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
      {items.length > 0 ? (
        <ul className="palette-results">
          {items.map((item, index) => {
            const showHeader = item.kind !== lastKind;
            lastKind = item.kind;
            const key =
              item.kind === "action"
                ? `action-${item.action.id}`
                : item.kind === "hit"
                  ? `hit-${item.hit.path}-${index}`
                  : item.kind === "date"
                    ? `date-${item.iso}`
                    : `${item.kind}-${item.title}`;
            const shortcut = item.kind === "action" ? getShortcut(shortcuts, item.action.id) : "";
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
                    <span className="palette-item-title">
                      {highlightMatch(item.action.label, trimmedQuery)}
                      {shortcut && (
                        <span className="palette-item-shortcut">{formatShortcut(shortcut)}</span>
                      )}
                    </span>
                  )}
                  {item.kind === "recent" && (
                    <span className="palette-item-title">
                      {highlightMatch(item.title, trimmedQuery)}
                    </span>
                  )}
                  {item.kind === "create" && (
                    <span className="palette-item-title">Crea «{item.title}»</span>
                  )}
                  {item.kind === "date" && (
                    <span className="palette-item-title">Vai al {formatPrettyDate(item.iso)}</span>
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
      ) : (
        trimmedQuery && <p className="palette-empty">Nessun risultato per «{trimmedQuery}»</p>
      )}
    </Modal>
  );
}
