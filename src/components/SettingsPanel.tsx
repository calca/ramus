import { useEffect, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

import {
  getSyncStatus,
  initGitSync,
  pickVaultFolder,
  setGitSyncInterval,
  setSearchShortcut as setSearchShortcutCommand,
  setTheme as setThemeCommand,
  setVaultPath,
  vaultStats,
} from "../lib/commands";
import { formatShortcut, normalizeShortcut } from "../lib/shortcut";
import { applyTheme } from "../lib/theme";
import type { Config, SyncStatus, Theme, VaultStats } from "../lib/types";
import { Modal } from "./Modal";

interface SettingsPanelProps {
  config: Config;
  onClose: () => void;
  onVaultChanged: (config: Config) => void;
  onThemeChanged: (config: Config) => void;
  onSearchShortcutChanged: (config: Config) => void;
  onGitSyncIntervalChanged: (config: Config) => void;
  onShowAbout: () => void;
}

const THEME_LABELS: Record<Theme, string> = {
  light: "Chiaro",
  dark: "Scuro",
  system: "Sistema",
};

const SYNC_INTERVAL_OPTIONS = [5, 10, 30, 60];

/** Polling leggero mentre il pannello Sync è aperto: si ferma alla
 * chiusura (l'effetto che lo avvia viene smontato insieme al pannello). */
const SYNC_STATUS_POLL_MS = 30_000;

export function SettingsPanel({
  config,
  onClose,
  onVaultChanged,
  onThemeChanged,
  onSearchShortcutChanged,
  onGitSyncIntervalChanged,
  onShowAbout,
}: SettingsPanelProps) {
  const [stats, setStats] = useState<VaultStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [recordingShortcut, setRecordingShortcut] = useState(false);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);
  const [syncBusy, setSyncBusy] = useState(false);

  useEffect(() => {
    void vaultStats()
      .then(setStats)
      .catch((err: unknown) => setError(String(err)));
  }, [config.vault_path]);

  useEffect(() => {
    const refresh = () => {
      void getSyncStatus()
        .then(setSyncStatus)
        .catch((err: unknown) => setError(String(err)));
    };
    refresh();
    const interval = setInterval(refresh, SYNC_STATUS_POLL_MS);
    return () => clearInterval(interval);
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

  const handleInitGitSync = async () => {
    setError(null);
    setSyncBusy(true);
    try {
      const status = await initGitSync();
      setSyncStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      setSyncBusy(false);
    }
  };

  const handleSyncIntervalChange = async (minutes: number) => {
    setError(null);
    try {
      const nextConfig = await setGitSyncInterval(minutes);
      onGitSyncIntervalChanged(nextConfig);
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

      <section className="settings-section">
        <h3>Sync</h3>
        {syncStatus?.enabled ? (
          <>
            <p className="settings-sync-status">
              {syncStatus.dirty
                ? "Modifiche in attesa del prossimo commit automatico"
                : "Tutto sincronizzato"}
              {syncStatus.last_commit_at !== null && (
                <> — ultimo commit {new Date(syncStatus.last_commit_at * 1000).toLocaleString()}</>
              )}
            </p>
            <label className="settings-sync-interval">
              Intervallo di sync
              <select
                value={config.git_sync_interval_minutes}
                onChange={(event) => void handleSyncIntervalChange(Number(event.target.value))}
              >
                {SYNC_INTERVAL_OPTIONS.map((minutes) => (
                  <option key={minutes} value={minutes}>
                    {minutes} minuti
                  </option>
                ))}
              </select>
            </label>
          </>
        ) : (
          <button type="button" disabled={syncBusy} onClick={() => void handleInitGitSync()}>
            Inizializza repository Git
          </button>
        )}
      </section>

      <button type="button" className="settings-about-link" onClick={onShowAbout}>
        Informazioni su Ramus
      </button>
    </Modal>
  );
}
