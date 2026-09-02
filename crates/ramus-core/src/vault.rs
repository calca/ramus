use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::block::{Block, Page};
use crate::error::CoreError;
use crate::journal_date::JournalDate;
use crate::parser;

pub struct Vault {
    pub root: PathBuf,
}

/// Conteggio delle note nel vault: usato solo per la sezione "info" delle
/// impostazioni, non per la vista journal (che resta su `list_journals`
/// paginato).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VaultStats {
    pub journal_count: usize,
    pub page_count: usize,
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

    /// Elenca i journal esistenti in ordine decrescente di data (più
    /// recente prima), strettamente precedenti a `before` (se `None`, si
    /// parte dal giorno più recente esistente). Non genera placeholder:
    /// un giorno senza file non compare. `limit` è clampato a
    /// [`MAX_LIST_JOURNALS_LIMIT`] per evitare richieste degeneri.
    pub fn list_journals(
        &self,
        before: Option<JournalDate>,
        limit: usize,
    ) -> Result<Vec<Page>, CoreError> {
        let limit = limit.min(MAX_LIST_JOURNALS_LIMIT);

        let mut dates = Vec::new();
        for entry in read_dir_entries(&self.root.join("journals"))? {
            let Some(date) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_suffix(".md"))
                .and_then(JournalDate::parse)
            else {
                continue;
            };
            if before.is_some_and(|before| date >= before) {
                continue;
            }
            dates.push(date);
        }
        dates.sort_by(|a, b| b.cmp(a));
        dates.truncate(limit);

        dates
            .into_iter()
            .map(|date| self.read_page(&Self::journal_relative_path(date)))
            .collect()
    }

    /// Conteggio di journal e pagine nel vault. Solo `read_dir`, nessun
    /// parsing del contenuto dei file.
    pub fn stats(&self) -> Result<VaultStats, CoreError> {
        let journal_count = read_dir_entries(&self.root.join("journals"))?
            .into_iter()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .and_then(|name| name.strip_suffix(".md"))
                    .is_some_and(|stem| JournalDate::parse(stem).is_some())
            })
            .count();
        let page_count = read_dir_entries(&self.root.join("pages"))?
            .into_iter()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.ends_with(".md"))
            })
            .count();
        Ok(VaultStats {
            journal_count,
            page_count,
        })
    }
}

const MAX_LIST_JOURNALS_LIMIT: usize = 90;

/// `fs::read_dir` che tratta una cartella mancante come vuota (sottocartella
/// del vault non ancora scritta, non un errore).
fn read_dir_entries(dir: &Path) -> Result<Vec<fs::DirEntry>, CoreError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(CoreError::Io {
                path: dir.to_path_buf(),
                source,
            })
        }
    };
    entries
        .map(|entry| {
            entry.map_err(|source| CoreError::Io {
                path: dir.to_path_buf(),
                source,
            })
        })
        .collect()
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

    fn write_journal(vault: &Vault, iso_date: &str, content: &str) {
        vault
            .write_page(&format!("journals/{iso_date}.md"), &[Block::new(content)])
            .unwrap();
    }

    #[test]
    fn list_journals_is_descending() {
        let dir = TempDir::new("list-descending");
        let vault = Vault::new(dir.path().to_path_buf());
        write_journal(&vault, "2026-01-01", "a");
        write_journal(&vault, "2026-01-03", "c");
        write_journal(&vault, "2026-01-02", "b");

        let dates: Vec<String> = vault
            .list_journals(None, 10)
            .unwrap()
            .into_iter()
            .map(|p| p.path.to_string_lossy().to_string())
            .collect();
        assert_eq!(
            dates,
            vec![
                "journals/2026-01-03.md",
                "journals/2026-01-02.md",
                "journals/2026-01-01.md",
            ]
        );
    }

    #[test]
    fn list_journals_skips_missing_days() {
        let dir = TempDir::new("list-skip-missing");
        let vault = Vault::new(dir.path().to_path_buf());
        write_journal(&vault, "2026-01-01", "a");
        write_journal(&vault, "2026-01-05", "e");

        let pages = vault.list_journals(None, 10).unwrap();
        assert_eq!(pages.len(), 2);
    }

    #[test]
    fn list_journals_respects_before_and_limit() {
        let dir = TempDir::new("list-before-limit");
        let vault = Vault::new(dir.path().to_path_buf());
        for day in 1..=5 {
            write_journal(&vault, &format!("2026-01-0{day}"), "x");
        }

        let before = JournalDate::parse("2026-01-04").unwrap();
        let pages = vault.list_journals(Some(before), 2).unwrap();
        let dates: Vec<String> = pages
            .into_iter()
            .map(|p| p.path.to_string_lossy().to_string())
            .collect();
        // prima di 2026-01-04, esclusa: 03, 02, 01 — limitate a 2.
        assert_eq!(
            dates,
            vec!["journals/2026-01-03.md", "journals/2026-01-02.md"]
        );
    }

    #[test]
    fn list_journals_ignores_non_date_filenames() {
        let dir = TempDir::new("list-ignore-junk");
        let vault = Vault::new(dir.path().to_path_buf());
        vault.ensure_exists().unwrap();
        write_journal(&vault, "2026-01-01", "a");
        std::fs::write(dir.path().join("journals/note.txt"), "not a journal").unwrap();
        std::fs::write(dir.path().join("journals/README.md"), "not a date").unwrap();

        let pages = vault.list_journals(None, 10).unwrap();
        assert_eq!(pages.len(), 1);
    }

    #[test]
    fn list_journals_on_empty_vault_is_empty_not_error() {
        let dir = TempDir::new("list-empty-vault");
        let vault = Vault::new(dir.path().to_path_buf());
        assert_eq!(vault.list_journals(None, 10).unwrap(), Vec::new());
    }

    #[test]
    fn stats_counts_journals_and_pages_ignoring_junk() {
        let dir = TempDir::new("stats-counts");
        let vault = Vault::new(dir.path().to_path_buf());
        write_journal(&vault, "2026-01-01", "a");
        write_journal(&vault, "2026-01-02", "b");
        vault
            .write_page("pages/uno.md", &[Block::new("x")])
            .unwrap();
        std::fs::write(dir.path().join("journals/README.md"), "not a date").unwrap();
        std::fs::write(dir.path().join("pages/note.txt"), "not markdown").unwrap();

        let stats = vault.stats().unwrap();
        assert_eq!(stats.journal_count, 2);
        assert_eq!(stats.page_count, 1);
    }

    #[test]
    fn stats_on_empty_vault_is_zero_not_error() {
        let dir = TempDir::new("stats-empty-vault");
        let vault = Vault::new(dir.path().to_path_buf());
        let stats = vault.stats().unwrap();
        assert_eq!(stats.journal_count, 0);
        assert_eq!(stats.page_count, 0);
    }
}
