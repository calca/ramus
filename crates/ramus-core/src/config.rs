use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::CoreError;

/// Tema dell'interfaccia. `System` segue `prefers-color-scheme`, senza che
/// il frontend debba leggerlo esplicitamente (vedi `assets/palette.css`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub vault_path: PathBuf,
    /// `default`: i `config.json` scritti da installazioni precedenti a
    /// questo campo non hanno la chiave `theme` — senza `default` la
    /// deserializzazione fallirebbe al primo avvio dopo l'aggiornamento.
    #[serde(default)]
    pub theme: Theme,
    /// Scorciatoie app-level configurabili, chiave = id azione stabile
    /// (es. "command_palette", "cheatsheet"), valore = stringa canonica
    /// ("Mod+K", "Mod" = Cmd su macOS, Ctrl altrove — normalizzato lato
    /// frontend). Rimpiazza il precedente `search_shortcut: String`
    /// (singola azione) — vedi `Config::load` per la migrazione dei
    /// `config.json` scritti dalla versione precedente.
    #[serde(default = "default_shortcuts")]
    pub shortcuts: HashMap<String, String>,
    /// Ogni quanti minuti il task di sync automatico (M3) controlla se
    /// committare. Stesso trattamento di `theme`/`search_shortcut` per
    /// compatibilità con `config.json` scritti prima di questo campo.
    #[serde(default = "default_git_sync_interval_minutes")]
    pub git_sync_interval_minutes: u32,
    /// Se `true`, i task `[ ] ` non fatti rimasti negli ultimi
    /// `task_rollover_days` giorni di journal vengono spostati
    /// automaticamente a oggi al cambio di giorno (M4).
    #[serde(default = "default_task_rollover_enabled")]
    pub task_rollover_enabled: bool,
    /// Ampiezza della finestra di scansione per lo spostamento
    /// automatico dei task, in giorni prima di oggi.
    #[serde(default = "default_task_rollover_days")]
    pub task_rollover_days: u32,
}

fn default_shortcuts() -> HashMap<String, String> {
    HashMap::from([
        ("command_palette".to_string(), "Mod+K".to_string()),
        ("cheatsheet".to_string(), "Mod+/".to_string()),
        ("focus_mode".to_string(), "Mod+.".to_string()),
        ("journal_prev_day".to_string(), "Mod+ArrowUp".to_string()),
        ("journal_next_day".to_string(), "Mod+ArrowDown".to_string()),
    ])
}

fn default_git_sync_interval_minutes() -> u32 {
    10
}

fn default_task_rollover_enabled() -> bool {
    true
}

fn default_task_rollover_days() -> u32 {
    7
}

/// Sposta `search_shortcut` (schema pre-M4) sotto `shortcuts.command_palette`
/// se il JSON ha ancora il campo vecchio e non ha già il nuovo. Ritorna
/// `true` se ha modificato qualcosa — il chiamante deve allora riscrivere
/// il file nel nuovo formato, per non ripetere la migrazione ad ogni avvio.
fn migrate_search_shortcut(value: &mut Value) -> bool {
    let Some(obj) = value.as_object_mut() else {
        return false;
    };
    if obj.contains_key("shortcuts") {
        return false;
    }
    let Some(old_shortcut) = obj.remove("search_shortcut") else {
        return false;
    };
    let mut shortcuts = serde_json::Map::new();
    shortcuts.insert("command_palette".to_string(), old_shortcut);
    obj.insert("shortcuts".to_string(), Value::Object(shortcuts));
    true
}

impl Config {
    pub fn default_vault_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Journal")
    }

    /// Percorso del file di configurazione dell'app (fuori dal vault: la
    /// configurazione non è una nota).
    pub fn config_file_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ramus")
            .join("config.json")
    }

    /// Carica `config.json`, migrando sul posto lo schema precedente
    /// (`search_shortcut: String`, singola azione) verso `shortcuts:
    /// HashMap<String, String>` se serve — vedi `migrate_search_shortcut`.
    /// Un `config.json` già nel nuovo formato, o uno privo di entrambi i
    /// campi (pre-M2), non viene mai riscritto qui: solo la migrazione
    /// esplicita lo fa, la stessa robustezza `#[serde(default = ...)]`
    /// già in uso per `theme`/`git_sync_interval_minutes` basta per gli
    /// altri casi.
    fn load(path: &Path) -> Result<Config, CoreError> {
        let text = fs::read_to_string(path).map_err(|source| CoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut value: Value =
            serde_json::from_str(&text).map_err(|e| CoreError::Config(e.to_string()))?;
        let migrated = migrate_search_shortcut(&mut value);

        let config: Config =
            serde_json::from_value(value).map_err(|e| CoreError::Config(e.to_string()))?;
        if migrated {
            config.save(path)?;
        }
        Ok(config)
    }

    fn save(&self, path: &Path) -> Result<(), CoreError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let text =
            serde_json::to_string_pretty(self).map_err(|e| CoreError::Config(e.to_string()))?;
        fs::write(path, text).map_err(|source| CoreError::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Carica la configurazione da disco, oppure la inizializza con il
    /// vault di default (zero attrito al primo avvio: nessun prompt).
    pub fn load_or_init() -> Result<Config, CoreError> {
        let path = Self::config_file_path();
        if path.exists() {
            Self::load(&path)
        } else {
            let config = Config {
                vault_path: Self::default_vault_path(),
                theme: Theme::default(),
                shortcuts: default_shortcuts(),
                git_sync_interval_minutes: default_git_sync_interval_minutes(),
                task_rollover_enabled: default_task_rollover_enabled(),
                task_rollover_days: default_task_rollover_days(),
            };
            config.save(&path)?;
            Ok(config)
        }
    }

    /// Aggiorna e persiste il vault path.
    pub fn set_vault_path(&mut self, vault_path: PathBuf) -> Result<(), CoreError> {
        self.vault_path = vault_path;
        self.save(&Self::config_file_path())
    }

    /// Aggiorna e persiste il tema.
    pub fn set_theme(&mut self, theme: Theme) -> Result<(), CoreError> {
        self.theme = theme;
        self.save(&Self::config_file_path())
    }

    /// Aggiorna e persiste la scorciatoia di una singola azione (chiave
    /// stabile, es. "command_palette"), lasciando le altre invariate.
    pub fn set_shortcut(&mut self, action_id: String, shortcut: String) -> Result<(), CoreError> {
        self.shortcuts.insert(action_id, shortcut);
        self.save(&Self::config_file_path())
    }

    /// Aggiorna e persiste l'intervallo del sync Git automatico.
    pub fn set_git_sync_interval_minutes(&mut self, minutes: u32) -> Result<(), CoreError> {
        self.git_sync_interval_minutes = minutes;
        self.save(&Self::config_file_path())
    }

    /// Aggiorna e persiste sia l'attivazione sia l'ampiezza (giorni) dello
    /// spostamento automatico dei task non fatti verso oggi — un solo
    /// command/setter per entrambi i campi, coerente con l'unica riga di
    /// impostazione che li mostra insieme in `SettingsPanel`.
    pub fn set_task_rollover(&mut self, enabled: bool, days: u32) -> Result<(), CoreError> {
        self.task_rollover_enabled = enabled;
        self.task_rollover_days = days;
        self.save(&Self::config_file_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_vault_path_is_under_home() {
        let path = Config::default_vault_path();
        assert!(path.ends_with("Journal"));
    }

    #[test]
    fn config_without_theme_field_defaults_to_system() {
        let json = r#"{"vault_path":"/home/x/Journal"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.theme, Theme::System);
    }

    #[test]
    fn theme_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&Theme::Dark).unwrap(), "\"dark\"");
    }

    #[test]
    fn config_without_shortcuts_field_defaults_to_all_registered_actions() {
        let json = r#"{"vault_path":"/home/x/Journal"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(
            config.shortcuts.get("command_palette"),
            Some(&"Mod+K".to_string())
        );
        assert_eq!(
            config.shortcuts.get("cheatsheet"),
            Some(&"Mod+/".to_string())
        );
        assert_eq!(
            config.shortcuts.get("focus_mode"),
            Some(&"Mod+.".to_string())
        );
        assert_eq!(
            config.shortcuts.get("journal_prev_day"),
            Some(&"Mod+ArrowUp".to_string())
        );
        assert_eq!(
            config.shortcuts.get("journal_next_day"),
            Some(&"Mod+ArrowDown".to_string())
        );
    }

    #[test]
    fn config_without_git_sync_interval_field_defaults_to_ten_minutes() {
        let json = r#"{"vault_path":"/home/x/Journal"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.git_sync_interval_minutes, 10);
    }

    #[test]
    fn config_without_task_rollover_fields_defaults_to_enabled_seven_days() {
        let json = r#"{"vault_path":"/home/x/Journal"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.task_rollover_enabled);
        assert_eq!(config.task_rollover_days, 7);
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let mut path = std::env::temp_dir();
            let unique = format!(
                "ramus-core-config-test-{label}-{}-{:?}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            path.push(unique);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn config_path(&self) -> PathBuf {
            self.0.join("config.json")
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn load_migrates_search_shortcut_into_shortcuts_and_rewrites_file() {
        let dir = TempDir::new("migrate-search-shortcut");
        let path = dir.config_path();
        fs::write(
            &path,
            r#"{"vault_path":"/home/x/Journal","search_shortcut":"Mod+P"}"#,
        )
        .unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(
            config.shortcuts.get("command_palette"),
            Some(&"Mod+P".to_string())
        );

        // Il file su disco è stato riscritto nel nuovo formato: un secondo
        // load non deve ripetere la migrazione né perdere il valore.
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains("search_shortcut"));
        let reloaded = Config::load(&path).unwrap();
        assert_eq!(
            reloaded.shortcuts.get("command_palette"),
            Some(&"Mod+P".to_string())
        );
    }

    #[test]
    fn load_without_search_shortcut_or_shortcuts_uses_defaults_without_rewriting() {
        let dir = TempDir::new("no-migration-needed");
        let path = dir.config_path();
        fs::write(&path, r#"{"vault_path":"/home/x/Journal"}"#).unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(
            config.shortcuts.get("command_palette"),
            Some(&"Mod+K".to_string())
        );

        // Nessuna migrazione necessaria: il file resta esattamente com'era.
        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, r#"{"vault_path":"/home/x/Journal"}"#);
    }

    #[test]
    fn load_already_in_new_format_is_left_untouched() {
        let dir = TempDir::new("already-new-format");
        let path = dir.config_path();
        let original =
            r#"{"vault_path":"/home/x/Journal","shortcuts":{"command_palette":"Mod+J"}}"#;
        fs::write(&path, original).unwrap();

        let config = Config::load(&path).unwrap();
        assert_eq!(
            config.shortcuts.get("command_palette"),
            Some(&"Mod+J".to_string())
        );

        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, original);
    }

    #[test]
    fn updating_one_shortcut_key_leaves_others_untouched_on_disk() {
        let dir = TempDir::new("set-shortcut");
        let path = dir.config_path();
        let mut config = Config {
            vault_path: dir.0.clone(),
            theme: Theme::default(),
            shortcuts: default_shortcuts(),
            git_sync_interval_minutes: default_git_sync_interval_minutes(),
            task_rollover_enabled: default_task_rollover_enabled(),
            task_rollover_days: default_task_rollover_days(),
        };
        config.save(&path).unwrap();

        config
            .shortcuts
            .insert("command_palette".to_string(), "Mod+J".to_string());
        config.save(&path).unwrap();

        let reloaded = Config::load(&path).unwrap();
        assert_eq!(
            reloaded.shortcuts.get("command_palette"),
            Some(&"Mod+J".to_string())
        );
        assert_eq!(
            reloaded.shortcuts.get("cheatsheet"),
            Some(&"Mod+/".to_string())
        );
    }
}
