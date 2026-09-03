import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";

import mascotteUrl from "../../assets/mascotte.svg";
import {
  getMcpInfo,
  getSyncStatus,
  initGitSync,
  pickVaultFolder,
  setGitRemote,
  setGitSyncInterval,
  setMcpEnabled,
  setShortcut as setShortcutCommand,
  setTaskRollover,
  setTheme as setThemeCommand,
  setVaultPath,
  vaultStats,
} from "../lib/commands";
import { SHORTCUT_ACTIONS, formatShortcut, getShortcut, normalizeShortcut } from "../lib/shortcut";
import { applyTheme } from "../lib/theme";
import type { Config, McpInfo, SyncStatus, Theme, VaultStats } from "../lib/types";
import { Modal } from "./Modal";

const REPO_URL = "https://github.com/calca/ramus";

interface SettingsPanelProps {
  config: Config;
  onClose: () => void;
  onVaultChanged: (config: Config) => void;
  onThemeChanged: (config: Config) => void;
  onShortcutChanged: (config: Config) => void;
  onGitSyncIntervalChanged: (config: Config) => void;
  onTaskRolloverChanged: (config: Config) => void;
  onMcpEnabledChanged: (config: Config) => void;
  /** Tab iniziale, es. "about" quando si apre da "Informazioni su Ramus"
   * nella command palette invece che dal bottone Impostazioni. */
  initialSection?: SettingsSectionId;
}

const THEME_LABELS: Record<Theme, string> = {
  light: "Chiaro",
  dark: "Scuro",
  system: "Sistema",
};

const SYNC_INTERVAL_OPTIONS = [5, 10, 30, 60];
const TASK_ROLLOVER_DAY_OPTIONS = [3, 7, 14, 30];

type SettingsSectionId = "vault" | "theme" | "shortcuts" | "task" | "mcp" | "sync" | "about";

const SETTINGS_SECTIONS: { id: SettingsSectionId; label: string }[] = [
  { id: "vault", label: "Vault" },
  { id: "theme", label: "Tema" },
  { id: "shortcuts", label: "Scorciatoie" },
  { id: "task", label: "Task" },
  { id: "mcp", label: "MCP" },
  { id: "sync", label: "Sync" },
  { id: "about", label: "Informazioni" },
];

/** Polling leggero mentre il pannello Sync è aperto: si ferma alla
 * chiusura (l'effetto che lo avvia viene smontato insieme al pannello). */
const SYNC_STATUS_POLL_MS = 30_000;

function syncStatusLabel(status: SyncStatus): string {
  switch (status.state) {
    case "conflict":
      return "Conflitto: sync automatica ferma";
    case "offline":
      return "Rete non raggiungibile, riprovo al prossimo giro";
    case "syncing":
      return "Sincronizzazione in corso…";
    default:
      return status.dirty
        ? "Modifiche in attesa del prossimo commit automatico"
        : "Tutto sincronizzato";
  }
}

/** Un solo bottone per l'intero flusso (attiva/collega/aggiorna remote)
 * invece di due azioni distinte da scoprire — vedi discussione utente su
 * "non è chiaro come collegare git". */
function syncActionLabel(status: SyncStatus | null): string {
  if (!status?.enabled) {
    return "Attiva sync";
  }
  return status.state === "noremote" ? "Collega remote" : "Aggiorna remote";
}

export function SettingsPanel({
  config,
  onClose,
  onVaultChanged,
  onThemeChanged,
  onShortcutChanged,
  onGitSyncIntervalChanged,
  onTaskRolloverChanged,
  onMcpEnabledChanged,
  initialSection,
}: SettingsPanelProps) {
  const [stats, setStats] = useState<VaultStats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [recordingActionId, setRecordingActionId] = useState<string | null>(null);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);
  const [syncBusy, setSyncBusy] = useState(false);
  const [remoteUrl, setRemoteUrl] = useState("");
  const [mcpInfo, setMcpInfo] = useState<McpInfo | null>(null);
  const [version, setVersion] = useState<string | null>(null);
  const [activeSection, setActiveSection] = useState<SettingsSectionId>(initialSection ?? "vault");

  useEffect(() => {
    void getVersion()
      .then(setVersion)
      .catch(() => {
        // Non blocca la pagina: la versione resta semplicemente assente.
      });
  }, []);

  useEffect(() => {
    void vaultStats()
      .then(setStats)
      .catch((err: unknown) => setError(String(err)));
  }, [config.vault_path]);

  // Solo all'apertura del pannello: a differenza dello stato di sync Git,
  // se il binario ramus-mcp compare/scompare non cambia mentre le
  // Impostazioni restano aperte, nessun polling necessario.
  useEffect(() => {
    void getMcpInfo()
      .then(setMcpInfo)
      .catch((err: unknown) => setError(String(err)));
  }, []);

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

  const handleSyncIntervalChange = async (minutes: number) => {
    setError(null);
    try {
      const nextConfig = await setGitSyncInterval(minutes);
      onGitSyncIntervalChanged(nextConfig);
    } catch (err) {
      setError(String(err));
    }
  };

  const handleTaskRolloverChange = async (enabled: boolean, days: number) => {
    setError(null);
    try {
      const nextConfig = await setTaskRollover(enabled, days);
      onTaskRolloverChanged(nextConfig);
    } catch (err) {
      setError(String(err));
    }
  };

  const handleMcpEnabledChange = async (enabled: boolean) => {
    setError(null);
    try {
      const nextConfig = await setMcpEnabled(enabled);
      onMcpEnabledChanged(nextConfig);
    } catch (err) {
      setError(String(err));
    }
  };

  /** Un solo bottone per l'intero flusso: attiva la sync se non lo è
   * ancora (con o senza URL nel campo — locale-soltanto è una scelta
   * valida), poi collega/aggiorna il remote se l'URL è compilato. */
  const handleSyncAction = async () => {
    setError(null);
    setSyncBusy(true);
    try {
      let status = syncStatus;
      if (!status?.enabled) {
        status = await initGitSync();
      }
      const url = remoteUrl.trim();
      if (url) {
        status = await setGitRemote(url);
        setRemoteUrl("");
      }
      setSyncStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      setSyncBusy(false);
    }
  };

  // Cattura in fase capture + stopPropagation: mentre si registra una
  // scorciatoia, Escape deve annullare la registrazione, non chiudere
  // anche l'intero pannello (il listener Escape di Modal è in bubble,
  // su window, e altrimenti la vedrebbe comunque).
  useEffect(() => {
    if (!recordingActionId) {
      return;
    }
    const actionId = recordingActionId;
    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        setRecordingActionId(null);
        return;
      }
      const shortcut = normalizeShortcut(event);
      if (shortcut) {
        setRecordingActionId(null);
        setError(null);
        void setShortcutCommand(actionId, shortcut)
          .then(onShortcutChanged)
          .catch((err: unknown) => setError(String(err)));
      }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, [recordingActionId, onShortcutChanged]);

  return (
    <Modal onClose={onClose} ariaLabel="Impostazioni">
      <header className="settings-panel-header">
        <h2>Impostazioni</h2>
        <button type="button" onClick={onClose} aria-label="Chiudi">
          ✕
        </button>
      </header>

      {error && <div className="banner banner-error">{error}</div>}

      <div className="settings-body">
        <nav className="settings-sidebar">
          {SETTINGS_SECTIONS.map((section) => (
            <button
              key={section.id}
              type="button"
              aria-current={activeSection === section.id ? "true" : undefined}
              onClick={() => setActiveSection(section.id)}
            >
              {section.label}
            </button>
          ))}
        </nav>

        <div className="settings-content">
        {activeSection === "vault" && (
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
        )}

        {activeSection === "theme" && (
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
        )}

        {activeSection === "shortcuts" && (
        <section className="settings-section">
          <h3>Scorciatoie</h3>
          <ul className="settings-shortcut-list">
            {SHORTCUT_ACTIONS.map((action) => (
              <li key={action.id}>
                <span>{action.label}</span>
                <button
                  type="button"
                  className="settings-shortcut-button"
                  onClick={() => setRecordingActionId(action.id)}
                >
                  {recordingActionId === action.id
                    ? "Premi una combinazione…"
                    : formatShortcut(getShortcut(config.shortcuts, action.id))}
                </button>
              </li>
            ))}
          </ul>
        </section>
        )}

        {activeSection === "task" && (
        <section className="settings-section">
          <h3>Task</h3>
          <label className="settings-task-rollover-toggle">
            <input
              type="checkbox"
              checked={config.task_rollover_enabled}
              onChange={(event) =>
                void handleTaskRolloverChange(event.target.checked, config.task_rollover_days)
              }
            />
            Sposta automaticamente a oggi i task non fatti rimasti indietro
          </label>
          {config.task_rollover_enabled && (
            <label className="settings-task-rollover-days">
              Considera gli ultimi
              <select
                value={config.task_rollover_days}
                onChange={(event) =>
                  void handleTaskRolloverChange(true, Number(event.target.value))
                }
              >
                {TASK_ROLLOVER_DAY_OPTIONS.map((days) => (
                  <option key={days} value={days}>
                    {days} giorni
                  </option>
                ))}
              </select>
            </label>
          )}
        </section>
        )}

        {activeSection === "mcp" && (
        <section className="settings-section">
          <h3>MCP</h3>
          <label className="settings-mcp-toggle">
            <input
              type="checkbox"
              checked={config.mcp_enabled}
              onChange={(event) => void handleMcpEnabledChange(event.target.checked)}
            />
            Abilita server MCP
          </label>
          {config.mcp_enabled ? (
            mcpInfo && (
              <>
                {mcpInfo.binary_found ? (
                  <>
                    <pre className="settings-mcp-snippet">{mcpInfo.config_snippet}</pre>
                    <p className="settings-mcp-help">
                      Incollalo in <code>.mcp.json</code> (Claude Code) o{" "}
                      <code>claude_desktop_config.json</code> (Claude Desktop). Riavvia il client
                      dopo una modifica.
                    </p>
                  </>
                ) : (
                  <p className="settings-mcp-help">
                    Binario <code>ramus-mcp</code> non trovato — esegui{" "}
                    <code>cargo build -p ramus-mcp</code> e riapri questa sezione.
                  </p>
                )}
              </>
            )
          ) : (
            <p className="settings-mcp-help">
              Il server MCP si rifiuta di avviarsi finché non lo riattivi qui.
            </p>
          )}
        </section>
        )}

        {activeSection === "sync" && (
        <section className="settings-section">
          <h3>Sync</h3>
          <p className="settings-sync-intro">
            Versiona il vault con Git. Lascia il campo vuoto per una
            cronologia solo locale, oppure incolla l'URL di un repository
            per sincronizzarlo fra dispositivi.
          </p>

          {syncStatus?.enabled && syncStatus.state === "conflict" && (
            <div className="banner banner-error">
              Il vault locale e quello remoto sono divergenti, serve
              intervento manuale: apri un terminale nel vault e risolvi con
              git.
            </div>
          )}

          {syncStatus?.enabled && (
            <p className="settings-sync-status">
              {syncStatusLabel(syncStatus)}
              {syncStatus.last_commit_at !== null && (
                <> — ultimo commit {new Date(syncStatus.last_commit_at * 1000).toLocaleString()}</>
              )}
            </p>
          )}

          <div className="settings-sync-remote">
            <input
              type="text"
              placeholder="git@github.com:utente/vault.git (opzionale)"
              value={remoteUrl}
              onChange={(event) => setRemoteUrl(event.target.value)}
            />
            <button
              type="button"
              disabled={syncBusy || (syncStatus?.enabled === true && !remoteUrl.trim())}
              onClick={() => void handleSyncAction()}
            >
              {syncActionLabel(syncStatus)}
            </button>
          </div>
          <p className="settings-sync-help">
            Su GitHub, GitLab o Bitbucket: apri il repository, premi "Code"
            (o "Clone"), copia l'URL SSH (consigliato, richiede una chiave
            già aggiunta al tuo account) o HTTPS, e incollalo qui sopra.
          </p>

          {syncStatus?.enabled && (
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
          )}
        </section>
        )}

        {activeSection === "about" && (
        <section className="settings-section">
          <div className="about-content">
            <img
              src={mascotteUrl}
              alt="Stecco, la mascotte di Ramus"
              className="about-mascotte"
              width={128}
            />
            <h3 className="about-name">Ramus</h3>
            {version && <p className="about-version">v{version}</p>}
            <p className="about-tagline">
              App desktop di journaling, outliner a blocchi su file markdown locali.
            </p>
            <button
              type="button"
              className="settings-about-link"
              onClick={() => void openUrl(REPO_URL)}
            >
              Codice sorgente
            </button>
          </div>
        </section>
        )}
        </div>
      </div>
    </Modal>
  );
}
