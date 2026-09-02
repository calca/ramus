use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub vault_path: PathBuf,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_vault_path_is_under_home() {
        let path = Config::default_vault_path();
        assert!(path.ends_with("Journal"));
    }
}
