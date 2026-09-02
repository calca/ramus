import { formatJournalHeader, journalDateFromPath } from "../lib/journal";
import type { Backlink } from "../lib/types";

interface BacklinksSectionProps {
  backlinks: Backlink[];
  onSelect: (title: string) => void;
}

/** Sezione "Backlink" sotto l'editor di una pagina: chi la linka, da
 * dove. Nessuna sezione visibile se non c'è nulla da mostrare (vedi
 * specs/M2/2026-09-02-pannello-backlink.DONE.md). I backlink da un journal non
 * sono cliccabili: non esiste ancora una vista/scroll-to-day isolata
 * per un singolo giorno. */
export function BacklinksSection({ backlinks, onSelect }: BacklinksSectionProps) {
  if (backlinks.length === 0) {
    return null;
  }

  return (
    <section className="page-view-backlinks">
      <h2>Backlink</h2>
      {backlinks.map((backlink, index) => {
        const isJournal = backlink.source_path.startsWith("journals/");
        const label = isJournal
          ? formatJournalHeader(journalDateFromPath(backlink.source_path))
          : (backlink.source_title ?? backlink.source_path);

        return (
          <div className="backlink-item" key={`${backlink.source_path}-${index}`}>
            {isJournal ? (
              <p className="backlink-item-source">{label}</p>
            ) : (
              <button
                type="button"
                className="backlink-item-source is-clickable"
                onClick={() => onSelect(backlink.source_title ?? backlink.source_path)}
              >
                {label}
              </button>
            )}
            <p className="backlink-item-content">{backlink.block_content}</p>
          </div>
        );
      })}
    </section>
  );
}
