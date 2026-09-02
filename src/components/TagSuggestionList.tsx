import { forwardRef, useEffect, useImperativeHandle, useState } from "react";
import type { SuggestionKeyDownProps } from "@tiptap/suggestion";

interface TagSuggestionListProps {
  items: string[];
  command: (tag: string) => void;
}

export interface TagSuggestionListHandle {
  onKeyDown: (props: SuggestionKeyDownProps) => boolean;
}

/** Popup dell'autocomplete per #tag: stesso pattern imperativo di
 * LinkSuggestionList (montato/smontato da tagAutocomplete.ts via
 * ReactRenderer), ma su una lista di stringhe invece di candidati con
 * kind "existing"/"create" — un tag non ha nulla da creare, vedi
 * specs/2026-09-02-autocomplete-tag.md. */
export const TagSuggestionList = forwardRef<TagSuggestionListHandle, TagSuggestionListProps>(
  function TagSuggestionList({ items, command }, ref) {
    const [selected, setSelected] = useState(0);

    useEffect(() => {
      setSelected(0);
    }, [items]);

    useImperativeHandle(ref, () => ({
      onKeyDown: ({ event }) => {
        if (items.length === 0) {
          return false;
        }
        if (event.key === "ArrowDown") {
          setSelected((index) => (index + 1) % items.length);
          return true;
        }
        if (event.key === "ArrowUp") {
          setSelected((index) => (index - 1 + items.length) % items.length);
          return true;
        }
        if (event.key === "Enter") {
          const item = items[selected];
          if (item) {
            command(item);
          }
          return true;
        }
        return false;
      },
    }));

    if (items.length === 0) {
      return null;
    }

    return (
      <div className="link-suggestion-list">
        {items.map((tag, index) => (
          <button
            type="button"
            key={tag}
            className={
              index === selected ? "link-suggestion-item is-selected" : "link-suggestion-item"
            }
            onMouseEnter={() => setSelected(index)}
            onClick={() => command(tag)}
          >
            {tag}
          </button>
        ))}
      </div>
    );
  },
);
