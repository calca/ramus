import { useEffect, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

import {
  pickVaultFolder,
  setSearchShortcut as setSearchShortcutCommand,
  setTheme as setThemeCommand,
  setVaultPath,
  vaultStats,
} from "../lib/commands";
import { formatShortcut, normalizeShortcut } from "../lib/shortcut";
import { applyTheme } from "../lib/theme";
import type { Config, Theme, VaultStats } from "../lib/types";
import { Modal } from "./Modal";

interface SettingsPanelProps {
  config: Config;
  onClose: () => void;
  onVaultChanged: (config: Config) => void;
  onThemeChanged: (config: Config) => void;
  onSearchShortcutChanged: (config: Config) => void;
  onShowAbout: () => void;
}

const THEME_LABELS: Record<Theme, string> = {
  light: "Chiaro",
  dark: "Scuro",
  system: "Sistema",
};

export function SettingsPanel({
  config,
  onClose,
  onVaultChanged,
  onThemeChanged,
  onSearchShortcutChanged,
  onShowAbout,
}: SettingsPanelProps) {
  const [stats, setStats] = useState<VaultStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [recordingShortcut, setRecordingShortcut] = useState(false);

  useEffect(() => {
    void vaultStats()
      .then(setStats)
      .catch((err: unknown) => setError(String(err)));
  }, [config.vault_path]);

  const handleChangeVault = async () => {
    setError(null);
    try {
      const picked = await pickVaultFolder();
      if (!picked || picked === config.vault_path) {
        return;
      }
      if (!window.confirm(`Apro il vault in ${picked}. Procedere?`)) {
        return;
      }
      setBusy(true);
      const nextConfig = await setVaultPath(picked);
      onVaultChanged(nextConfig);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleOpenInFileManager = async () => {
    setError(null);
    try {
      await revealItemInDir(config.vault_path);
    } catch (err) {
      setError(String(err));
    }
  };

  const handleThemeChange = async (theme: Theme) => {
    setError(null);
    try {
      const nextConfig = await setThemeCommand(theme);
      applyTheme(theme);
      onThemeChanged(nextConfig);
    } catch (err) {
      setError(String(err));
    }
  };

  // Cattura in fase capture + stopPropagation: mentre si registra una
  // scorciatoia, Escape deve annullare la registrazione, non chiudere
  // anche l'intero pannello (il listener Escape di Modal è in bubble,
  // su window, e altrimenti la vedrebbe comunque).
  useEffect(() => {
    if (!recordingShortcut) {
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        setRecordingShortcut(false);
        return;
      }
      const shortcut = normalizeShortcut(event);
      if (shortcut) {
        setRecordingShortcut(false);
        setError(null);
        void setSearchShortcutCommand(shortcut)
          .then(onSearchShortcutChanged)
          .catch((err: unknown) => setError(String(err)));
      }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, [recordingShortcut, onSearchShortcutChanged]);

  return (
    <Modal onClose={onClose} ariaLabel="Impostazioni">
      <header className="settings-panel-header">
        <h2>Impostazioni</h2>
        <button type="button" onClick={onClose} aria-label="Chiudi">
          ✕
        </button>
      </header>

      {error && <div className="banner banner-error">{error}</div>}

      <section className="settings-section">
        <h3>Vault</h3>
        <p className="settings-vault-path">{config.vault_path}</p>
        <div className="settings-vault-actions">
          <button type="button" disabled={busy} onClick={() => void handleChangeVault()}>
            Cambia
          </button>
          <button type="button" onClick={() => void handleOpenInFileManager()}>
            Apri nel file manager
          </button>
        </div>
        {stats && (
          <p className="settings-vault-stats">
            {stats.journal_count} journal, {stats.page_count} pagine
          </p>
        )}
      </section>

      <section className="settings-section">
        <h3>Tema</h3>
        <div className="settings-theme-options">
          {(Object.keys(THEME_LABELS) as Theme[]).map((option) => (
            <label key={option}>
              <input
                type="radio"
                name="theme"
                value={option}
                checked={config.theme === option}
                onChange={() => void handleThemeChange(option)}
              />
              {THEME_LABELS[option]}
            </label>
          ))}
        </div>
      </section>

      <section className="settings-section">
        <h3>Ricerca</h3>
        <button
          type="button"
          className="settings-shortcut-button"
          onClick={() => setRecordingShortcut(true)}
        >
          {recordingShortcut ? "Premi una combinazione…" : formatShortcut(config.search_shortcut)}
        </button>
      </section>

      <button type="button" className="settings-about-link" onClick={onShowAbout}>
        Informazioni su Ramus
      </button>
    </Modal>
  );
}
