import { useEffect } from "react";
import type { ReactNode } from "react";

interface ModalProps {
  onClose: () => void;
  ariaLabel: string;
  children: ReactNode;
  /** Classe aggiuntiva sul pannello, oltre a "settings-panel" — es.
   * "palette-panel" per CommandPalette, che ha un padding e un'altezza
   * diversi dal dialog di Impostazioni (auto invece di fissa: si
   * restringe/allarga con il numero di risultati). */
  panelClassName?: string;
}

/** Backdrop + pannello centrato, chiusura con Escape o click fuori.
 * Condiviso da SettingsPanel, CommandPalette e Cheatsheet: stessa
 * meccanica identica. */
export function Modal({ onClose, ariaLabel, children, panelClassName }: ModalProps) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  return (
    <div className="settings-backdrop" onClick={onClose}>
      <div
        className={panelClassName ? `settings-panel ${panelClassName}` : "settings-panel"}
        role="dialog"
        aria-modal="true"
        aria-label={ariaLabel}
        onClick={(event) => event.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}
