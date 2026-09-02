import { formatIsoDate, isSameDay } from "../lib/journal";

interface DayNavProps {
  currentDate: Date;
  onNavigate: (date: Date) => void;
}

export function DayNav({ currentDate, onNavigate }: DayNavProps) {
  const today = new Date();
  const prev = new Date(currentDate);
  prev.setDate(prev.getDate() - 1);
  const next = new Date(currentDate);
  next.setDate(next.getDate() + 1);

  return (
    <nav className="day-nav">
      <button type="button" onClick={() => onNavigate(prev)} aria-label="Giorno precedente">
        ←
      </button>
      <span className={isSameDay(currentDate, today) ? "day-nav-date day-nav-date-today" : "day-nav-date"}>
        {formatIsoDate(currentDate)}
      </span>
      <button type="button" onClick={() => onNavigate(next)} aria-label="Giorno successivo">
        →
      </button>
      <button type="button" onClick={() => onNavigate(today)} disabled={isSameDay(currentDate, today)}>
        Oggi
      </button>
    </nav>
  );
}
