use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::block::{Block, Page};
use crate::error::CoreError;
use crate::frontmatter;
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

/// Voce dell'elenco pagine, per l'autocomplete di `[[link]]`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PageSummary {
    pub slug: String,
    /// Titolo dal front-matter, o lo slug stesso se assente.
    pub title: String,
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
        let (front_matter, body) = frontmatter::split_front_matter(&text);
        let title = front_matter.and_then(frontmatter::extract_title);
        let blocks = parser::parse(body)?;
        Ok(Page {
            path: PathBuf::from(relative_path),
            title,
            blocks,
        })
    }

    /// Il command Tauri passa solo `blocks`, mai il front-matter: se il
    /// file esistente ne ha uno (titolo di una pagina), va preservato
    /// intatto — altrimenti la prima battitura in una pagina cancellerebbe
    /// silenziosamente il suo titolo.
    pub fn write_page(&self, relative_path: &str, blocks: &[Block]) -> Result<(), CoreError> {
        let abs = self.resolve(relative_path)?;
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let existing_front_matter = fs::read_to_string(&abs)
            .ok()
            .and_then(|text| frontmatter::split_front_matter(&text).0.map(str::to_string));
        let mut out = existing_front_matter.unwrap_or_default();
        out.push_str(&parser::render(blocks));
        fs::write(&abs, out).map_err(|source| CoreError::Io { path: abs, source })
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

    /// Apre la pagina identificata da `name`, creandola se non esiste
    /// ancora. Lo slug del file è `slugify(name)`; se il file va creato,
    /// il front-matter iniziale è `title: {name}` — il testo esatto
    /// passato, non lo slug (permette a `[[link]]` di mostrare un titolo
    /// leggibile invece di uno slug grezzo).
    pub fn open_page(&self, name: &str) -> Result<Page, CoreError> {
        let relative_path = Self::page_relative_path(name);
        let abs = self.resolve(&relative_path)?;
        if !abs.exists() {
            if let Some(parent) = abs.parent() {
                fs::create_dir_all(parent).map_err(|source| CoreError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            let front_matter = format!("---\ntitle: {name}\n---\n");
            let body = parser::render(&[Block::new("")]);
            fs::write(&abs, format!("{front_matter}{body}"))
                .map_err(|source| CoreError::Io { path: abs, source })?;
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

    /// Pagine esistenti in `pages/`, slug + titolo, ordinate per titolo.
    /// Titolo assente (nessun front-matter, o front-matter senza
    /// `title`) ricade sullo slug. File illeggibili vengono saltati
    /// invece di far fallire l'intera lista: è usata per suggerimenti di
    /// autocomplete, non deve rompersi per una pagina sola corrotta.
    pub fn list_pages(&self) -> Result<Vec<PageSummary>, CoreError> {
        let mut pages: Vec<PageSummary> = read_dir_entries(&self.root.join("pages"))?
            .into_iter()
            .filter_map(|entry| {
                let slug = entry.file_name().to_str()?.strip_suffix(".md")?.to_string();
                let text = fs::read_to_string(entry.path()).ok()?;
                let title = frontmatter::split_front_matter(&text)
                    .0
                    .and_then(frontmatter::extract_title)
                    .unwrap_or_else(|| slug.clone());
                Some(PageSummary { slug, title })
            })
            .collect();
        pages.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(pages)
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

    #[test]
    fn open_page_creates_file_with_title_in_front_matter() {
        let dir = TempDir::new("open-page-creates");
        let vault = Vault::new(dir.path().to_path_buf());
        let page = vault.open_page("Progetto X").unwrap();
        assert_eq!(page.title, Some("Progetto X".to_string()));
        assert_eq!(page.blocks, vec![Block::new("")]);
        assert_eq!(page.path, PathBuf::from("pages/progetto-x.md"));
    }

    #[test]
    fn open_page_on_existing_page_does_not_overwrite() {
        let dir = TempDir::new("open-page-existing");
        let vault = Vault::new(dir.path().to_path_buf());
        let first = vault.open_page("Progetto X").unwrap();
        vault
            .write_page(&first.path.to_string_lossy(), &[Block::new("scritto")])
            .unwrap();
        let second = vault.open_page("Progetto X").unwrap();
        assert_eq!(second.title, Some("Progetto X".to_string()));
        assert_eq!(second.blocks, vec![Block::new("scritto")]);
    }

    #[test]
    fn write_page_preserves_existing_front_matter() {
        let dir = TempDir::new("write-preserves-front-matter");
        let vault = Vault::new(dir.path().to_path_buf());
        let page = vault.open_page("Progetto X").unwrap();
        vault
            .write_page(
                &page.path.to_string_lossy(),
                &[Block::new("nuovo contenuto")],
            )
            .unwrap();
        let reloaded = vault.read_page(&page.path.to_string_lossy()).unwrap();
        assert_eq!(reloaded.title, Some("Progetto X".to_string()));
        assert_eq!(reloaded.blocks, vec![Block::new("nuovo contenuto")]);
    }

    #[test]
    fn write_page_on_journal_is_unaffected_by_front_matter_logic() {
        let dir = TempDir::new("write-journal-unaffected");
        let vault = Vault::new(dir.path().to_path_buf());
        let today = vault.open_today().unwrap();
        vault
            .write_page(&today.path.to_string_lossy(), &[Block::new("scritto")])
            .unwrap();
        let reloaded = vault.read_page(&today.path.to_string_lossy()).unwrap();
        assert_eq!(reloaded.title, None);
        assert_eq!(reloaded.blocks, vec![Block::new("scritto")]);
    }

    #[test]
    fn list_pages_returns_slug_and_title_sorted_by_title() {
        let dir = TempDir::new("list-pages-sorted");
        let vault = Vault::new(dir.path().to_path_buf());
        vault.open_page("Zeta").unwrap();
        vault.open_page("Alpha").unwrap();
        let pages = vault.list_pages().unwrap();
        assert_eq!(
            pages,
            vec![
                PageSummary {
                    slug: "alpha".to_string(),
                    title: "Alpha".to_string(),
                },
                PageSummary {
                    slug: "zeta".to_string(),
                    title: "Zeta".to_string(),
                },
            ]
        );
    }

    #[test]
    fn list_pages_falls_back_to_slug_without_title() {
        let dir = TempDir::new("list-pages-no-title");
        let vault = Vault::new(dir.path().to_path_buf());
        vault
            .write_page("pages/senza-titolo.md", &[Block::new("x")])
            .unwrap();
        let pages = vault.list_pages().unwrap();
        assert_eq!(
            pages,
            vec![PageSummary {
                slug: "senza-titolo".to_string(),
                title: "senza-titolo".to_string(),
            }]
        );
    }

    #[test]
    fn list_pages_on_empty_vault_is_empty_not_error() {
        let dir = TempDir::new("list-pages-empty");
        let vault = Vault::new(dir.path().to_path_buf());
        assert_eq!(vault.list_pages().unwrap(), Vec::new());
    }
}
