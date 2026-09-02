import { forwardRef, useEffect, useImperativeHandle, useState } from "react";
import type { SuggestionKeyDownProps } from "@tiptap/suggestion";

export interface LinkCandidate {
  kind: "existing" | "create";
  title: string;
}

interface LinkSuggestionListProps {
  items: LinkCandidate[];
  command: (item: LinkCandidate) => void;
}

export interface LinkSuggestionListHandle {
  onKeyDown: (props: SuggestionKeyDownProps) => boolean;
}

/** Popup dell'autocomplete per [[link]]: montato/smontato imperativamente
 * da linkAutocomplete.ts via ReactRenderer, non da un albero React normale
 * (il ciclo di vita è guidato dai callback di @tiptap/suggestion). */
export const LinkSuggestionList = forwardRef<LinkSuggestionListHandle, LinkSuggestionListProps>(
  function LinkSuggestionList({ items, command }, ref) {
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
        {items.map((item, index) => (
          <button
            type="button"
            key={`${item.kind}-${item.title}`}
            className={
              index === selected ? "link-suggestion-item is-selected" : "link-suggestion-item"
            }
            onMouseEnter={() => setSelected(index)}
            onClick={() => command(item)}
          >
            {item.kind === "create" ? `Crea «${item.title}»` : item.title}
          </button>
        ))}
      </div>
    );
  },
);
