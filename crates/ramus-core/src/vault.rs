use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::block::{Block, Page};
use crate::error::CoreError;
use crate::journal_date::JournalDate;
use crate::parser;

pub struct Vault {
    pub root: PathBuf,
}

impl Vault {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Crea le sottocartelle `journals/` e `pages/` se non esistono già.
    pub fn ensure_exists(&self) -> Result<(), CoreError> {
        for sub in ["journals", "pages"] {
            let dir = self.root.join(sub);
            fs::create_dir_all(&dir).map_err(|source| CoreError::Io { path: dir, source })?;
        }
        Ok(())
    }

    /// Path relativo del journal per una data, es. "journals/2026-09-02.md".
    pub fn journal_relative_path(date: JournalDate) -> String {
        format!("journals/{date}.md")
    }

    /// Path relativo di una pagina a partire dal suo nome, es.
    /// "pages/nome-pagina.md".
    pub fn page_relative_path(name: &str) -> String {
        format!("pages/{}.md", slugify(name))
    }

    /// Risolve un path relativo al vault in un path assoluto, rifiutando
    /// qualunque tentativo di uscire dalla radice del vault (`..`, path
    /// assoluti). Il frontend passa solo path relativi: questa è l'unica
    /// porta d'ingresso al filesystem del vault.
    pub fn resolve(&self, relative_path: &str) -> Result<PathBuf, CoreError> {
        let rel = Path::new(relative_path);
        for component in rel.components() {
            match component {
                Component::Normal(_) => {}
                _ => return Err(CoreError::InvalidPath(relative_path.to_string())),
            }
        }
        Ok(self.root.join(rel))
    }

    pub fn read_page(&self, relative_path: &str) -> Result<Page, CoreError> {
        let abs = self.resolve(relative_path)?;
        let text = fs::read_to_string(&abs).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                CoreError::PageNotFound(abs.clone())
            } else {
                CoreError::Io { path: abs, source }
            }
        })?;
        let blocks = parser::parse(&text)?;
        Ok(Page {
            path: PathBuf::from(relative_path),
            blocks,
        })
    }

    pub fn write_page(&self, relative_path: &str, blocks: &[Block]) -> Result<(), CoreError> {
        let abs = self.resolve(relative_path)?;
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let text = parser::render(blocks);
        fs::write(&abs, text).map_err(|source| CoreError::Io { path: abs, source })
    }

    /// Apre il journal di oggi, creandolo con un blocco vuoto se non esiste.
    pub fn open_today(&self) -> Result<Page, CoreError> {
        let relative_path = Self::journal_relative_path(JournalDate::today());
        let abs = self.resolve(&relative_path)?;
        if !abs.exists() {
            self.write_page(&relative_path, &[Block::new("")])?;
        }
        self.read_page(&relative_path)
    }
}

/// Converte un nome in slug: minuscolo, spazi sostituiti da trattini.
pub fn slugify(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile_free::TempDir;

    mod tempfile_free {
        use std::path::PathBuf;

        /// Piccola directory temporanea usa-e-getta per i test, senza
        /// aggiungere la dipendenza `tempfile`.
        pub struct TempDir(pub PathBuf);

        impl TempDir {
            pub fn new(label: &str) -> Self {
                let mut path = std::env::temp_dir();
                let unique = format!(
                    "ramus-core-test-{label}-{}-{:?}",
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

            pub fn path(&self) -> &std::path::Path {
                &self.0
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    #[test]
    fn ensure_exists_creates_subfolders() {
        let dir = TempDir::new("ensure-exists");
        let vault = Vault::new(dir.path().to_path_buf());
        vault.ensure_exists().unwrap();
        assert!(dir.path().join("journals").is_dir());
        assert!(dir.path().join("pages").is_dir());
    }

    #[test]
    fn resolve_rejects_parent_traversal() {
        let dir = TempDir::new("resolve-traversal");
        let vault = Vault::new(dir.path().to_path_buf());
        assert!(matches!(
            vault.resolve("../outside.md"),
            Err(CoreError::InvalidPath(_))
        ));
        assert!(matches!(
            vault.resolve("journals/../../outside.md"),
            Err(CoreError::InvalidPath(_))
        ));
    }

    #[test]
    fn resolve_rejects_absolute_path() {
        let dir = TempDir::new("resolve-absolute");
        let vault = Vault::new(dir.path().to_path_buf());
        assert!(matches!(
            vault.resolve("/etc/passwd"),
            Err(CoreError::InvalidPath(_))
        ));
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = TempDir::new("write-read");
        let vault = Vault::new(dir.path().to_path_buf());
        let blocks = vec![Block::new("ciao"), Block::new("mondo")];
        vault.write_page("pages/test.md", &blocks).unwrap();
        let page = vault.read_page("pages/test.md").unwrap();
        assert_eq!(page.blocks, blocks);
    }

    #[test]
    fn read_missing_page_is_not_found() {
        let dir = TempDir::new("read-missing");
        let vault = Vault::new(dir.path().to_path_buf());
        assert!(matches!(
            vault.read_page("pages/missing.md"),
            Err(CoreError::PageNotFound(_))
        ));
    }

    #[test]
    fn open_today_creates_file_once() {
        let dir = TempDir::new("open-today");
        let vault = Vault::new(dir.path().to_path_buf());
        let first = vault.open_today().unwrap();
        assert_eq!(first.blocks, vec![Block::new("")]);

        vault
            .write_page(&first.path.to_string_lossy(), &[Block::new("scritto")])
            .unwrap();
        let second = vault.open_today().unwrap();
        assert_eq!(second.blocks, vec![Block::new("scritto")]);
    }

    #[test]
    fn slugify_lowercases_and_dashes_spaces() {
        assert_eq!(slugify("Nome Pagina"), "nome-pagina");
        assert_eq!(slugify("  Molti   Spazi  "), "molti-spazi");
    }

    #[test]
    fn page_relative_path_uses_slug() {
        assert_eq!(
            Vault::page_relative_path("Nome Pagina"),
            "pages/nome-pagina.md"
        );
    }
}
