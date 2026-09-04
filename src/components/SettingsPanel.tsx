import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import type { TFunction } from "i18next";
import { Trans, useTranslation } from "react-i18next";

import mascotteUrl from "../../assets/mascotte.svg";
import { applyLocale } from "../i18n";
import { translateError } from "../lib/errors";
import {
  getMcpInfo,
  getSyncStatus,
  initGitSync,
  pickVaultFolder,
  setGitRemote,
  setGitSyncInterval,
  setLocale as setLocaleCommand,
  setMcpEnabled,
  setShortcut as setShortcutCommand,
  setTaskRollover,
  setTheme as setThemeCommand,
  setVaultPath,
  vaultStats,
} from "../lib/commands";
import { SHORTCUT_ACTIONS, formatShortcut, getShortcut, normalizeShortcut } from "../lib/shortcut";
import { applyTheme } from "../lib/theme";
import type { Config, Locale, McpInfo, SyncStatus, Theme, VaultStats } from "../lib/types";
import { Modal } from "./Modal";

const REPO_URL = "https://github.com/calca/ramus";

interface SettingsPanelProps {
  config: Config;
  onClose: () => void;
  onVaultChanged: (config: Config) => void;
  onThemeChanged: (config: Config) => void;
  onLocaleChanged: (config: Config) => void;
  onShortcutChanged: (config: Config) => void;
  onGitSyncIntervalChanged: (config: Config) => void;
  onTaskRolloverChanged: (config: Config) => void;
  onMcpEnabledChanged: (config: Config) => void;
  /** Tab iniziale, es. "about" quando si apre da "Informazioni su Ramus"
   * nella command palette invece che dal bottone Impostazioni. */
  initialSection?: SettingsSectionId;
}

const SYNC_INTERVAL_OPTIONS = [5, 10, 30, 60];
const TASK_ROLLOVER_DAY_OPTIONS = [3, 7, 14, 30];

type SettingsSectionId = "vault" | "theme" | "locale" | "shortcuts" | "task" | "mcp" | "sync" | "about";

const SETTINGS_SECTION_IDS: SettingsSectionId[] = [
  "vault",
  "theme",
  "locale",
  "shortcuts",
  "task",
  "mcp",
  "sync",
  "about",
];

/** Polling leggero mentre il pannello Sync è aperto: si ferma alla
 * chiusura (l'effetto che lo avvia viene smontato insieme al pannello). */
const SYNC_STATUS_POLL_MS = 30_000;

// syncStatusLabel/syncActionLabel non sono componenti React: prendono `t`
// come parametro (dalla chiamata a useTranslation() del componente) invece
// di chiamare l'hook loro stesse — non è legale chiamare un hook fuori da
// un componente/hook React.
function syncStatusLabel(status: SyncStatus, t: TFunction): string {
  switch (status.state) {
    case "conflict":
      return t("settings.sync.status.conflict");
    case "offline":
      return t("settings.sync.status.offline");
    case "syncing":
      return t("settings.sync.status.syncing");
    default:
      return status.dirty ? t("settings.sync.status.dirty") : t("settings.sync.status.clean");
  }
}

/** Un solo bottone per l'intero flusso (attiva/collega/aggiorna remote)
 * invece di due azioni distinte da scoprire — vedi discussione utente su
 * "non è chiaro come collegare git". */
function syncActionLabel(status: SyncStatus | null, t: TFunction): string {
  if (!status?.enabled) {
    return t("settings.sync.action.activate");
  }
  return status.state === "noremote"
    ? t("settings.sync.action.connect")
    : t("settings.sync.action.update");
}

export function SettingsPanel({
  config,
  onClose,
  onVaultChanged,
  onThemeChanged,
  onLocaleChanged,
  onShortcutChanged,
  onGitSyncIntervalChanged,
  onTaskRolloverChanged,
  onMcpEnabledChanged,
  initialSection,
}: SettingsPanelProps) {
  const { t } = useTranslation();
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
      .catch((err: unknown) => setError(translateError(err)));
  }, [config.vault_path]);

  // Solo all'apertura del pannello: a differenza dello stato di sync Git,
  // se il binario ramus-mcp compare/scompare non cambia mentre le
  // Impostazioni restano aperte, nessun polling necessario.
  useEffect(() => {
    void getMcpInfo()
      .then(setMcpInfo)
      .catch((err: unknown) => setError(translateError(err)));
  }, []);

  useEffect(() => {
    const refresh = () => {
      void getSyncStatus()
        .then(setSyncStatus)
        .catch((err: unknown) => setError(translateError(err)));
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
      if (!window.confirm(t("settings.vault.confirmChange", { path: picked }))) {
        return;
      }
      setBusy(true);
      const nextConfig = await setVaultPath(picked);
      onVaultChanged(nextConfig);
    } catch (err) {
      setError(translateError(err));
    } finally {
      setBusy(false);
    }
  };

  const handleOpenInFileManager = async () => {
    setError(null);
    try {
      await revealItemInDir(config.vault_path);
    } catch (err) {
      setError(translateError(err));
    }
  };

  const handleThemeChange = async (theme: Theme) => {
    setError(null);
    try {
      const nextConfig = await setThemeCommand(theme);
      applyTheme(theme);
      onThemeChanged(nextConfig);
    } catch (err) {
      setError(translateError(err));
    }
  };

  const handleLocaleChange = async (locale: Locale) => {
    setError(null);
    try {
      const nextConfig = await setLocaleCommand(locale);
      applyLocale(locale);
      onLocaleChanged(nextConfig);
    } catch (err) {
      setError(translateError(err));
    }
  };

  const handleSyncIntervalChange = async (minutes: number) => {
    setError(null);
    try {
      const nextConfig = await setGitSyncInterval(minutes);
      onGitSyncIntervalChanged(nextConfig);
    } catch (err) {
      setError(translateError(err));
    }
  };

  const handleTaskRolloverChange = async (enabled: boolean, days: number) => {
    setError(null);
    try {
      const nextConfig = await setTaskRollover(enabled, days);
      onTaskRolloverChanged(nextConfig);
    } catch (err) {
      setError(translateError(err));
    }
  };

  const handleMcpEnabledChange = async (enabled: boolean) => {
    setError(null);
    try {
      const nextConfig = await setMcpEnabled(enabled);
      onMcpEnabledChanged(nextConfig);
    } catch (err) {
      setError(translateError(err));
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
      setError(translateError(err));
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
          .catch((err: unknown) => setError(translateError(err)));
      }
    };
    window.addEventListener("keydown", onKeyDown, { capture: true });
    return () => window.removeEventListener("keydown", onKeyDown, { capture: true });
  }, [recordingActionId, onShortcutChanged]);

  // Non dei `const` a livello di modulo (come lo era THEME_LABELS prima di
  // questa spec): devono chiamare t(), quindi vanno calcolati dentro al
  // componente, ricalcolati automaticamente ad ogni render — react-i18next
  // ri-renderizza da solo ogni componente con useTranslation() quando la
  // lingua cambia (vedi src/i18n/index.ts).
  const THEME_LABELS: Record<Theme, string> = {
    light: t("settings.theme.light"),
    dark: t("settings.theme.dark"),
    system: t("settings.theme.system"),
  };
  const LOCALE_LABELS: Record<Locale, string> = {
    it: t("settings.locale.it"),
    en: t("settings.locale.en"),
    system: t("settings.locale.system"),
  };
  const SECTION_LABELS: Record<SettingsSectionId, string> = {
    vault: t("settings.sections.vault"),
    theme: t("settings.sections.theme"),
    locale: t("settings.sections.locale"),
    shortcuts: t("settings.sections.shortcuts"),
    task: t("settings.sections.task"),
    mcp: t("settings.sections.mcp"),
    sync: t("settings.sections.sync"),
    about: t("settings.sections.about"),
  };

  return (
    <Modal onClose={onClose} ariaLabel={t("settings.title")}>
      <header className="settings-panel-header">
        <h2>{t("settings.title")}</h2>
        <button type="button" onClick={onClose} aria-label={t("common.close")}>
          ✕
        </button>
      </header>

      {error && <div className="banner banner-error">{error}</div>}

      <div className="settings-body">
        <nav className="settings-sidebar">
          {SETTINGS_SECTION_IDS.map((id) => (
            <button
              key={id}
              type="button"
              aria-current={activeSection === id ? "true" : undefined}
              onClick={() => setActiveSection(id)}
            >
              {SECTION_LABELS[id]}
            </button>
          ))}
        </nav>

        <div className="settings-content">
        {activeSection === "vault" && (
        <section className="settings-section">
          <h3>{SECTION_LABELS.vault}</h3>
          <p className="settings-vault-path">{config.vault_path}</p>
          <div className="settings-vault-actions">
            <button type="button" disabled={busy} onClick={() => void handleChangeVault()}>
              {t("settings.vault.change")}
            </button>
            <button type="button" onClick={() => void handleOpenInFileManager()}>
              {t("settings.vault.openInFileManager")}
            </button>
          </div>
          {stats && (
            <p className="settings-vault-stats">
              {t("settings.vault.stats", {
                journals: stats.journal_count,
                pages: stats.page_count,
              })}
            </p>
          )}
        </section>
        )}

        {activeSection === "theme" && (
        <section className="settings-section">
          <h3>{SECTION_LABELS.theme}</h3>
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

        {activeSection === "locale" && (
        <section className="settings-section">
          <h3>{SECTION_LABELS.locale}</h3>
          <div className="settings-theme-options">
            {(Object.keys(LOCALE_LABELS) as Locale[]).map((option) => (
              <label key={option}>
                <input
                  type="radio"
                  name="locale"
                  value={option}
                  checked={config.locale === option}
                  onChange={() => void handleLocaleChange(option)}
                />
                {LOCALE_LABELS[option]}
              </label>
            ))}
          </div>
        </section>
        )}

        {activeSection === "shortcuts" && (
        <section className="settings-section">
          <h3>{SECTION_LABELS.shortcuts}</h3>
          <ul className="settings-shortcut-list">
            {SHORTCUT_ACTIONS.map((action) => (
              <li key={action.id}>
                <span>{t(action.labelKey)}</span>
                <button
                  type="button"
                  className="settings-shortcut-button"
                  onClick={() => setRecordingActionId(action.id)}
                >
                  {recordingActionId === action.id
                    ? t("settings.shortcuts.recording")
                    : formatShortcut(getShortcut(config.shortcuts, action.id))}
                </button>
              </li>
            ))}
          </ul>
        </section>
        )}

        {activeSection === "task" && (
        <section className="settings-section">
          <h3>{SECTION_LABELS.task}</h3>
          <label className="settings-task-rollover-toggle">
            <input
              type="checkbox"
              checked={config.task_rollover_enabled}
              onChange={(event) =>
                void handleTaskRolloverChange(event.target.checked, config.task_rollover_days)
              }
            />
            {t("settings.task.rolloverToggle")}
          </label>
          {config.task_rollover_enabled && (
            <label className="settings-task-rollover-days">
              {t("settings.task.rolloverDaysLabel")}
              <select
                value={config.task_rollover_days}
                onChange={(event) =>
                  void handleTaskRolloverChange(true, Number(event.target.value))
                }
              >
                {TASK_ROLLOVER_DAY_OPTIONS.map((days) => (
                  <option key={days} value={days}>
                    {t("settings.task.days", { count: days })}
                  </option>
                ))}
              </select>
            </label>
          )}
        </section>
        )}

        {activeSection === "mcp" && (
        <section className="settings-section">
          <h3>{SECTION_LABELS.mcp}</h3>
          <label className="settings-mcp-toggle">
            <input
              type="checkbox"
              checked={config.mcp_enabled}
              onChange={(event) => void handleMcpEnabledChange(event.target.checked)}
            />
            {t("settings.mcp.enable")}
          </label>
          {config.mcp_enabled ? (
            mcpInfo && (
              <>
                {mcpInfo.binary_found ? (
                  <>
                    <pre className="settings-mcp-snippet">{mcpInfo.config_snippet}</pre>
                    <p className="settings-mcp-help">
                      <Trans i18nKey="settings.mcp.help.pasteSnippet" components={[<code key="0" />, <code key="1" />]} />
                    </p>
                  </>
                ) : (
                  <p className="settings-mcp-help">
                    <Trans i18nKey="settings.mcp.help.notFound" components={[<code key="0" />, <code key="1" />]} />
                  </p>
                )}
              </>
            )
          ) : (
            <p className="settings-mcp-help">{t("settings.mcp.help.disabled")}</p>
          )}
        </section>
        )}

        {activeSection === "sync" && (
        <section className="settings-section">
          <h3>{SECTION_LABELS.sync}</h3>
          <p className="settings-sync-intro">{t("settings.sync.intro")}</p>

          {syncStatus?.enabled && syncStatus.state === "conflict" && (
            <div className="banner banner-error">{t("settings.sync.conflictBanner")}</div>
          )}

          {syncStatus?.enabled && (
            <p className="settings-sync-status">
              {syncStatusLabel(syncStatus, t)}
              {syncStatus.last_commit_at !== null && (
                <>
                  {t("settings.sync.lastCommit", {
                    datetime: new Date(syncStatus.last_commit_at * 1000).toLocaleString(),
                  })}
                </>
              )}
            </p>
          )}

          <div className="settings-sync-remote">
            <input
              type="text"
              placeholder={t("settings.sync.remotePlaceholder")}
              value={remoteUrl}
              onChange={(event) => setRemoteUrl(event.target.value)}
            />
            <button
              type="button"
              disabled={syncBusy || (syncStatus?.enabled === true && !remoteUrl.trim())}
              onClick={() => void handleSyncAction()}
            >
              {syncActionLabel(syncStatus, t)}
            </button>
          </div>
          <p className="settings-sync-help">{t("settings.sync.help")}</p>

          {syncStatus?.enabled && (
            <label className="settings-sync-interval">
              {t("settings.sync.intervalLabel")}
              <select
                value={config.git_sync_interval_minutes}
                onChange={(event) => void handleSyncIntervalChange(Number(event.target.value))}
              >
                {SYNC_INTERVAL_OPTIONS.map((minutes) => (
                  <option key={minutes} value={minutes}>
                    {t("settings.sync.minutes", { count: minutes })}
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
              alt={t("settings.about.mascotteAlt")}
              className="about-mascotte"
              width={128}
            />
            <h3 className="about-name">Ramus</h3>
            {version && <p className="about-version">{t("settings.about.version", { version })}</p>}
            <p className="about-tagline">{t("settings.about.tagline")}</p>
            <button
              type="button"
              className="settings-about-link"
              onClick={() => void openUrl(REPO_URL)}
            >
              {t("settings.about.sourceCode")}
            </button>
          </div>
        </section>
        )}
        </div>
      </div>
    </Modal>
  );
}
