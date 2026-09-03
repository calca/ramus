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
const PRETTY_DATE_FORMATTER = new Intl.DateTimeFormat("it-IT", { day: "numeric", month: "long" });
const PRETTY_DATE_WITH_YEAR_FORMATTER = new Intl.DateTimeFormat("it-IT", {
  day: "numeric",
  month: "long",
  year: "numeric",
});

function daysBetween(fromIso: string, toIso: string): number {
  const msPerDay = 24 * 60 * 60 * 1000;
  const diff = parseIsoDate(toIso).getTime() - parseIsoDate(fromIso).getTime();
  // Math.round, non una divisione secca: un giorno di cambio ora
  // legale/solare dura 23 o 25 ore, non esattamente 24.
  return Math.round(diff / msPerDay);
}

function relativeLabel(iso: string): string | null {
  const days = daysBetween(iso, formatIsoDate(new Date()));
  if (days === 0) return "Oggi";
  if (days === 1) return "Ieri";
  if (days >= 2 && days <= 6) return `${days} giorni fa`;
  return null;
}

/** "2 settembre" nell'anno corrente, "2 settembre 2025" altrimenti: l'anno
 * è rumore quando è ovvio, utile quando non lo è. */
export function formatPrettyDate(iso: string): string {
  const date = parseIsoDate(iso);
  const isCurrentYear = date.getFullYear() === new Date().getFullYear();
  return (isCurrentYear ? PRETTY_DATE_FORMATTER : PRETTY_DATE_WITH_YEAR_FORMATTER).format(date);
}

const ISO_DATE_PATTERN = /^(\d{4})-(\d{1,2})-(\d{1,2})$/;
const IT_DATE_PATTERN = /^(\d{1,2})[/-](\d{1,2})[/-](\d{4})$/;

/** Riconosce una data digitata a mano in formato ISO (YYYY-MM-DD) o
 * italiano (DD/MM/YYYY, anche con "-" come separatore) — usata dalla
 * command palette al posto del vecchio date-picker nativo. `null` se il
 * testo non è nel formato atteso, se la data non esiste (es. 31 aprile:
 * `new Date` la farebbe scivolare silenziosamente all'1 maggio, da qui
 * il controllo di round-trip) o se è nel futuro (stesso limite del
 * vecchio picker, `max` = oggi). */
export function parseTypedDate(input: string): string | null {
  const trimmed = input.trim();
  let year: number | undefined;
  let month: number | undefined;
  let day: number | undefined;

  const iso = ISO_DATE_PATTERN.exec(trimmed);
  if (iso) {
    [year, month, day] = [Number(iso[1]), Number(iso[2]), Number(iso[3])];
  } else {
    const it = IT_DATE_PATTERN.exec(trimmed);
    if (it) {
      [day, month, year] = [Number(it[1]), Number(it[2]), Number(it[3])];
    }
  }
  if (year === undefined || month === undefined || day === undefined) {
    return null;
  }

  const date = new Date(year, month - 1, day);
  if (date.getFullYear() !== year || date.getMonth() !== month - 1 || date.getDate() !== day) {
    return null;
  }
  const result = formatIsoDate(date);
  return result > formatIsoDate(new Date()) ? null : result;
}

function capitalizeFirst(text: string): string {
  return text.length > 0 ? text[0].toUpperCase() + text.slice(1) : text;
}

/** Header leggibile per una sezione di journal: relativo negli ultimi
 * sette giorni ("Oggi 2 settembre", "3 giorni fa 30 agosto"), altrimenti
 * assoluto ("Mercoledì 19 agosto"). Solo la prima lettera è maiuscola:
 * i nomi dei mesi in italiano restano minuscoli anche a metà stringa. */
export function formatJournalHeader(iso: string): string {
  const label = relativeLabel(iso) ?? WEEKDAY_FORMATTER.format(parseIsoDate(iso));
  return capitalizeFirst(`${label} ${formatPrettyDate(iso)}`);
}
