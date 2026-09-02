use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

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
    /// Scorciatoia per aprire il pannello di ricerca, es. "Mod+K" ("Mod" =
    /// Cmd su macOS, Ctrl altrove — normalizzato lato frontend). Stesso
    /// trattamento di `theme`: `default` per compatibilità con i
    /// `config.json` scritti prima di questo campo.
    #[serde(default = "default_search_shortcut")]
    pub search_shortcut: String,
}

fn default_search_shortcut() -> String {
    "Mod+K".to_string()
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

    fn load(path: &Path) -> Result<Config, CoreError> {
        let text = fs::read_to_string(path).map_err(|source| CoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        serde_json::from_str(&text).map_err(|e| CoreError::Config(e.to_string()))
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
                search_shortcut: default_search_shortcut(),
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

    /// Aggiorna e persiste la scorciatoia di ricerca.
    pub fn set_search_shortcut(&mut self, shortcut: String) -> Result<(), CoreError> {
        self.search_shortcut = shortcut;
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
    fn config_without_search_shortcut_field_defaults_to_mod_k() {
        let json = r#"{"vault_path":"/home/x/Journal"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.search_shortcut, "Mod+K");
    }
}
