import { useEffect, useState } from "react";

import { listOpenTasks } from "../lib/commands";
import { formatJournalHeader, journalDateFromPath } from "../lib/journal";
import type { TaskHit } from "../lib/types";
import { Modal } from "./Modal";

interface OpenTasksPanelProps {
  onClose: () => void;
  /** Naviga alla pagina sorgente del task — solo navigazione, non un modo
   * di segnare il task fatto da qui (vedi spec: coerente con "sola
   * lettura, poi vai al blocco" già scelto per backlink/ricerca). */
  onSelectTask: (task: TaskHit) => void;
}

/** Label leggibile per un task: header di journal formattato se
 * `kind === "journal"` (stesso trattamento di JournalSection), altrimenti
 * titolo della pagina (con lo stesso fallback sul path già usato per i
 * risultati di ricerca quando il front-matter non ha un titolo). */
function labelFor(task: TaskHit): string {
  if (task.kind === "journal") {
    return formatJournalHeader(journalDateFromPath(task.path));
  }
  return task.title ?? task.path.replace(/^pages\//, "").replace(/\.md$/, "");
}

export function OpenTasksPanel({ onClose, onSelectTask }: OpenTasksPanelProps) {
  const [tasks, setTasks] = useState<TaskHit[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    listOpenTasks()
      .then((result) => {
        if (!cancelled) {
          setTasks(result);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(String(err));
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <Modal onClose={onClose} ariaLabel="Task aperti" panelClassName="tasks-panel">
      <header className="settings-panel-header">
        <h2>Task aperti</h2>
        <button type="button" onClick={onClose} aria-label="Chiudi">
          ✕
        </button>
      </header>

      {error && <p className="tasks-panel-empty">{error}</p>}
      {!error && tasks === null && <p className="tasks-panel-empty">Caricamento…</p>}
      {!error && tasks !== null && tasks.length === 0 && (
        <p className="tasks-panel-empty">Nessun task aperto.</p>
      )}
      {!error && tasks !== null && tasks.length > 0 && (
        <ul className="tasks-panel-list">
          {tasks.map((task) => (
            <li key={`${task.path}:${task.content}`}>
              <button type="button" className="tasks-panel-item" onClick={() => onSelectTask(task)}>
                <span className="tasks-panel-item-content">{task.content.replace(/^\[ \] /, "")}</span>
                <span className="tasks-panel-item-source">{labelFor(task)}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </Modal>
  );
}
