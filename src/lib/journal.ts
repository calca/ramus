// Convenzioni del vault lato frontend: solo calcolo di path relativi, mai
// accesso al filesystem. Deve rispecchiare `JournalDate` e
// `Vault::journal_relative_path` in ramus-core.

/** Data locale in formato ISO 8601 (YYYY-MM-DD). Mai `toISOString`: è UTC e
 * disallineerebbe il giorno mostrato rispetto al calendario locale usato
 * dal core Rust (`chrono::Local`). */
export function formatIsoDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

export function journalRelativePath(date: Date): string {
  return `journals/${formatIsoDate(date)}.md`;
}

export function addDays(date: Date, delta: number): Date {
  const next = new Date(date);
  next.setDate(next.getDate() + delta);
  return next;
}

export function isSameDay(a: Date, b: Date): boolean {
  return formatIsoDate(a) === formatIsoDate(b);
}
