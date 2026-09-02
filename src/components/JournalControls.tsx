import { useState } from "react";

import { formatIsoDate } from "../lib/journal";

interface JournalControlsProps {
  onToday: () => void;
  onJumpToDate: (iso: string) => void;
}

const TODAY_ISO = formatIsoDate(new Date());

export function JournalControls({ onToday, onJumpToDate }: JournalControlsProps) {
  const [pickerValue, setPickerValue] = useState(TODAY_ISO);

  return (
    <div className="journal-controls">
      <input
        type="date"
        className="journal-date-picker"
        value={pickerValue}
        max={TODAY_ISO}
        aria-label="Vai a data"
        onChange={(event) => {
          const iso = event.currentTarget.value;
          setPickerValue(iso);
          if (iso) {
            onJumpToDate(iso);
          }
        }}
      />
      <button type="button" onClick={onToday}>
        Oggi
      </button>
    </div>
  );
}
