import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";

import mascotteUrl from "../../assets/mascotte.svg";
import { Modal } from "./Modal";

const REPO_URL = "https://github.com/calca/ramus";

interface AboutPanelProps {
  onClose: () => void;
}

export function AboutPanel({ onClose }: AboutPanelProps) {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    void getVersion()
      .then(setVersion)
      .catch(() => {
        // Non blocca la pagina: la versione resta semplicemente assente.
      });
  }, []);

  return (
    <Modal onClose={onClose} ariaLabel="Informazioni su Ramus">
      <header className="settings-panel-header">
        <h2>Informazioni su Ramus</h2>
        <button type="button" onClick={onClose} aria-label="Chiudi">
          ✕
        </button>
      </header>

      <div className="about-content">
        <img src={mascotteUrl} alt="Stecco, la mascotte di Ramus" className="about-mascotte" width={128} height={128} />
        <h3 className="about-name">Ramus</h3>
        {version && <p className="about-version">v{version}</p>}
        <p className="about-tagline">
          App desktop di journaling, outliner a blocchi su file markdown locali.
        </p>
        <button type="button" className="settings-about-link" onClick={() => void openUrl(REPO_URL)}>
          Codice sorgente
        </button>
      </div>
    </Modal>
  );
}
