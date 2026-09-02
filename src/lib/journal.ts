// Convenzioni del vault lato frontend: solo calcolo di path/date, mai
// accesso al filesystem. Deve rispecchiare `JournalDate` e
// `Vault::journal_relative_path` in ramus-core. Le date sono trattate come
// stringhe ISO 8601 (YYYY-MM-DD): il confronto lessicografico coincide con
// l'ordine cronologico, quindi non serve costruire `Date` per ordinare o
// confrontare prima/dopo.

const JOURNAL_PATH_PATTERN = /^journals\/(\d{4}-\d{2}-\d{2})\.md$/;

/** Data locale in formato ISO 8601 (YYYY-MM-DD). Mai `toISOString`: è UTC e
 * disallineerebbe il giorno mostrato rispetto al calendario locale usato
 * dal core Rust (`chrono::Local`). */
export function formatIsoDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

/** Estrae la data ISO dal path relativo di un journal (es.
 * "journals/2026-09-02.md" -> "2026-09-02"). */
export function journalDateFromPath(path: string): string {
  const match = JOURNAL_PATH_PATTERN.exec(path);
  return match ? match[1] : "";
}

/** Interpreta una data ISO come `Date` locale (mai `new Date(iso)`: quello
 * è UTC e può far scivolare il giorno della settimana mostrato). */
function parseIsoDate(iso: string): Date {
  const [year, month, day] = iso.split("-").map(Number);
  return new Date(year, month - 1, day);
}

const WEEKDAY_FORMATTER = new Intl.DateTimeFormat("it-IT", { weekday: "long" });

/** Header leggibile per una sezione di journal, es. "mercoledì 2026-09-02". */
export function formatJournalHeader(iso: string): string {
  return `${WEEKDAY_FORMATTER.format(parseIsoDate(iso))} ${iso}`;
}
