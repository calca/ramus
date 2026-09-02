import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";

import { search } from "../lib/commands";
import { formatJournalHeader, journalDateFromPath } from "../lib/journal";
import type { SearchHit } from "../lib/types";
import { Modal } from "./Modal";

interface SearchPanelProps {
  onClose: () => void;
  onSelect: (hit: SearchHit) => void;
}

const DEBOUNCE_MS = 250;

/** Pannello di ricerca full-text (M2): granularità per pagina/giorno
 * intero, mai per blocco — vedi specs/2026-09-02-ricerca-full-text.md. */
export function SearchPanel({ onClose, onSelect }: SearchPanelProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchHit[]>([]);
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    setSelected(0);
    if (timerRef.current !== null) {
      clearTimeout(timerRef.current);
    }
    const trimmed = query.trim();
    if (!trimmed) {
      setResults([]);
      return;
    }
    timerRef.current = setTimeout(() => {
      void search(trimmed).then(setResults);
    }, DEBOUNCE_MS);
    return () => {
      if (timerRef.current !== null) {
        clearTimeout(timerRef.current);
      }
    };
  }, [query]);

  const handleKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>) => {
    if (results.length === 0) {
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelected((index) => (index + 1) % results.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelected((index) => (index - 1 + results.length) % results.length);
    } else if (event.key === "Enter") {
      event.preventDefault();
      const hit = results[selected];
      if (hit) {
        onSelect(hit);
      }
    }
  };

  return (
    <Modal onClose={onClose} ariaLabel="Ricerca">
      <input
        ref={inputRef}
        type="text"
        className="search-input"
        placeholder="Cerca nel vault…"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        onKeyDown={handleKeyDown}
      />
      {results.length > 0 && (
        <ul className="search-results">
          {results.map((hit, index) => (
            <li key={`${hit.path}-${index}`}>
              <button
                type="button"
                className={index === selected ? "search-result is-selected" : "search-result"}
                onMouseEnter={() => setSelected(index)}
                onClick={() => onSelect(hit)}
              >
                <span className="search-result-title">
                  {hit.kind === "journal"
                    ? formatJournalHeader(journalDateFromPath(hit.path))
                    : (hit.title ?? hit.path)}
                </span>
                {hit.snippet_html && (
                  <span
                    className="search-result-snippet"
                    dangerouslySetInnerHTML={{ __html: hit.snippet_html }}
                  />
                )}
              </button>
            </li>
          ))}
        </ul>
      )}
    </Modal>
  );
}
