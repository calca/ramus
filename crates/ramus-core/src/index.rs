//! Indice SQLite di pagine/blocchi/link/tag (vedi
//! specs/M2/2026-09-02-indice-sqlite.DONE.md). Derivato e rigenerabile dai file
//! markdown (SPEC.md, principio #1): se il file `.sqlite3` viene cancellato,
//! `sync` lo ricostruisce identico dal vault.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};

use crate::block::Block;
use crate::error::CoreError;
use crate::journal_date::JournalDate;
use crate::vault::{slugify, Vault};

const SCHEMA_VERSION: i64 = 1;

pub struct Index {
    conn: Connection,
}

/// Una riga di risultato per "chi linka a questa pagina".
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Backlink {
    pub source_path: String,
    pub source_title: Option<String>,
    pub block_content: String,
}

/// Cosa ha effettivamente cambiato un `Index::sync`: guida `SearchIndex`
/// (indice tantivy, `search.rs`) senza che debba tenere una propria
/// contabilità di mtime — vedi specs/M2/2026-09-02-ricerca-full-text.DONE.md.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncOutcome {
    pub refreshed: Vec<String>,
    pub removed: Vec<String>,
}

impl Index {
    /// Apre (creando se serve) `<vault_root>/.ramus/index.sqlite3` e allinea
    /// lo schema. Non sincronizza il contenuto: chiamare [`Index::sync`]
    /// dopo l'apertura.
    pub fn open(vault_root: &Path) -> Result<Self, CoreError> {
        let dir = vault_root.join(".ramus");
        fs::create_dir_all(&dir).map_err(|source| CoreError::Io {
            path: dir.clone(),
            source,
        })?;
        let conn = Connection::open(dir.join("index.sqlite3"))?;
        conn.pragma_update(None, "foreign_keys", true)?;
        ensure_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Allinea l'indice al contenuto attuale del vault: legge solo l'mtime
    /// di ogni file (`stat`, non il contenuto) e reindicizza esclusivamente
    /// ciò che manca o è cambiato da allora, rimuovendo le righe di file
    /// non più presenti su disco. Un indice vuoto è il caso degenere dello
    /// stesso algoritmo: tutto risulta "assente" e viene reindicizzato.
    pub fn sync(&self, vault: &Vault) -> Result<SyncOutcome, CoreError> {
        let disk_pages = list_disk_pages(vault)?;

        let mut stmt = self.conn.prepare("SELECT path, mtime FROM pages")?;
        let indexed: HashMap<String, i64> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<_, rusqlite::Error>>()?;
        drop(stmt);

        let mut refreshed = Vec::new();
        for (relative_path, mtime) in &disk_pages {
            let unchanged = indexed.get(relative_path).is_some_and(|prev| prev == mtime);
            if !unchanged {
                self.refresh_page(vault, relative_path)?;
                refreshed.push(relative_path.clone());
            }
        }

        let disk_paths: HashSet<&str> = disk_pages.iter().map(|(path, _)| path.as_str()).collect();
        let mut removed = Vec::new();
        for path in indexed.keys() {
            if !disk_paths.contains(path.as_str()) {
                self.conn
                    .execute("DELETE FROM pages WHERE path = ?1", params![path])?;
                removed.push(path.clone());
            }
        }

        Ok(SyncOutcome { refreshed, removed })
    }

    /// Rilegge una pagina dal disco e sostituisce le sue righe nell'indice
    /// (pagina, blocchi, link, tag). Usata sia da [`Index::sync`] per i
    /// file nuovi/cambiati sia dai command dopo ogni scrittura, così
    /// l'indice resta coerente durante la sessione senza un `sync`
    /// completo a ogni battitura.
    pub fn refresh_page(&self, vault: &Vault, relative_path: &str) -> Result<(), CoreError> {
        let abs = vault.resolve(relative_path)?;
        let mtime = file_mtime(&abs)?;
        let page = vault.read_page(relative_path)?;
        let kind = if relative_path.starts_with("journals/") {
            "journal"
        } else {
            "page"
        };

        self.conn.execute(
            "INSERT INTO pages (path, kind, title, mtime) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET
                 kind = excluded.kind,
                 title = excluded.title,
                 mtime = excluded.mtime",
            params![relative_path, kind, page.title, mtime],
        )?;
        let page_id: i64 = self.conn.query_row(
            "SELECT id FROM pages WHERE path = ?1",
            params![relative_path],
            |row| row.get(0),
        )?;

        // I blocchi vecchi vengono cancellati e reinseriti: link/tag
        // seguono a cascata (`ON DELETE CASCADE`), niente diffing riga per
        // riga necessario per un contenuto che è comunque riletto per
        // intero.
        self.conn
            .execute("DELETE FROM blocks WHERE page_id = ?1", params![page_id])?;

        for (position, block) in flatten_blocks(&page.blocks).into_iter().enumerate() {
            self.conn.execute(
                "INSERT INTO blocks (page_id, position, content) VALUES (?1, ?2, ?3)",
                params![page_id, position as i64, block.content],
            )?;
            let block_id = self.conn.last_insert_rowid();

            for target_title in extract_links(&block.content) {
                self.conn.execute(
                    "INSERT INTO links (source_block_id, target_title) VALUES (?1, ?2)",
                    params![block_id, target_title],
                )?;
            }
            for tag in extract_tags(&block.content) {
                self.conn.execute(
                    "INSERT INTO tags (block_id, tag) VALUES (?1, ?2)",
                    params![block_id, tag],
                )?;
            }
        }

        Ok(())
    }

    /// Blocchi che linkano a `target_title`, in qualunque pagina. Il
    /// confronto è via `slugify` (non FK diretta): "Progetto X" e
    /// "progetto x" risolvono alla stessa pagina.
    pub fn find_backlinks(&self, target_title: &str) -> Result<Vec<Backlink>, CoreError> {
        let target_slug = slugify(target_title);
        let mut stmt = self.conn.prepare(
            "SELECT links.target_title, blocks.content, pages.path, pages.title
             FROM links
             JOIN blocks ON blocks.id = links.source_block_id
             JOIN pages ON pages.id = blocks.page_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;

        let mut backlinks = Vec::new();
        for row in rows {
            let (link_target, block_content, source_path, source_title) = row?;
            if slugify(&link_target) == target_slug {
                backlinks.push(Backlink {
                    source_path,
                    source_title,
                    block_content,
                });
            }
        }
        Ok(backlinks)
    }

    /// Tag distinti presenti nel vault, in ordine alfabetico.
    pub fn list_tags(&self) -> Result<Vec<String>, CoreError> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT tag FROM tags ORDER BY tag")?;
        let tags = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, rusqlite::Error>>()?;
        Ok(tags)
    }
}

fn ensure_schema(conn: &Connection) -> Result<(), CoreError> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if current != SCHEMA_VERSION {
        conn.execute_batch(
            "DROP TABLE IF EXISTS tags;
             DROP TABLE IF EXISTS links;
             DROP TABLE IF EXISTS blocks;
             DROP TABLE IF EXISTS pages;
             CREATE TABLE pages (
                 id INTEGER PRIMARY KEY,
                 path TEXT NOT NULL UNIQUE,
                 kind TEXT NOT NULL,
                 title TEXT,
                 mtime INTEGER NOT NULL
             );
             CREATE TABLE blocks (
                 id INTEGER PRIMARY KEY,
                 page_id INTEGER NOT NULL REFERENCES pages(id) ON DELETE CASCADE,
                 position INTEGER NOT NULL,
                 content TEXT NOT NULL
             );
             CREATE TABLE links (
                 id INTEGER PRIMARY KEY,
                 source_block_id INTEGER NOT NULL REFERENCES blocks(id) ON DELETE CASCADE,
                 target_title TEXT NOT NULL
             );
             CREATE TABLE tags (
                 id INTEGER PRIMARY KEY,
                 block_id INTEGER NOT NULL REFERENCES blocks(id) ON DELETE CASCADE,
                 tag TEXT NOT NULL
             );",
        )?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    Ok(())
}

/// Elenca i file markdown di `journals/` e `pages/` con il loro path
/// relativo e `mtime` (secondi epoch). Solo `stat`, nessuna lettura di
/// contenuto: usato da `sync` per decidere cosa è cambiato.
fn list_disk_pages(vault: &Vault) -> Result<Vec<(String, i64)>, CoreError> {
    let mut result = Vec::new();
    for (sub, is_journal) in [("journals", true), ("pages", false)] {
        let dir = vault.root.join(sub);
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => continue,
            Err(source) => return Err(CoreError::Io { path: dir, source }),
        };
        for entry in entries {
            let entry = entry.map_err(|source| CoreError::Io {
                path: dir.clone(),
                source,
            })?;
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            let Some(stem) = name.strip_suffix(".md") else {
                continue;
            };
            if is_journal && JournalDate::parse(stem).is_none() {
                continue;
            }
            let relative_path = format!("{sub}/{name}");
            let mtime = file_mtime(&entry.path())?;
            result.push((relative_path, mtime));
        }
    }
    Ok(result)
}

fn file_mtime(path: &Path) -> Result<i64, CoreError> {
    let metadata = fs::metadata(path).map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let modified = metadata.modified().map_err(|source| CoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let secs = modified
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(secs as i64)
}

/// Appiattisce l'albero dei blocchi in ordine di visita pre-order: la
/// gerarchia non serve nell'indice (deciso in
/// specs/M2/2026-09-02-indice-sqlite.DONE.md), solo un ordine stabile. `pub(crate)`:
/// riusata anche da `search.rs` per lo stesso motivo (niente duplicazione,
/// vedi specs/M2/2026-09-02-ricerca-full-text.DONE.md).
pub(crate) fn flatten_blocks(blocks: &[Block]) -> Vec<&Block> {
    fn walk<'a>(blocks: &'a [Block], flat: &mut Vec<&'a Block>) {
        for block in blocks {
            flat.push(block);
            walk(&block.children, flat);
        }
    }
    let mut flat = Vec::new();
    walk(blocks, &mut flat);
    flat
}

/// Estrae i titoli `[[link]]` da un blocco. Scritta a mano (niente `regex`,
/// vedi CLAUDE.md): stessi confini di `[^\]]+` — testo non vuoto e privo di
/// `]` fra `[[` e `]]`.
pub fn extract_links(content: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut i = 0;
    while let Some(open_rel) = content[i..].find("[[") {
        let start = i + open_rel + 2;
        let Some(close_rel) = content[start..].find(']') else {
            break;
        };
        let close = start + close_rel;
        if close > start && content.as_bytes().get(close + 1) == Some(&b']') {
            links.push(content[start..close].to_string());
            i = close + 2;
        } else {
            i = start;
        }
    }
    links
}

/// Estrae i tag `#tag` da un blocco. Stesso set di caratteri della
/// decorazione frontend (`linkTagHighlight.ts`): `/#[\w-]+/` senza flag
/// `u`, cioè `\w` ASCII (alfanumerico + `_`), più `-`.
pub fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let chars: Vec<(usize, char)> = content.char_indices().collect();
    let mut idx = 0;
    while idx < chars.len() {
        let (_, ch) = chars[idx];
        if ch == '#' {
            let start = idx + 1;
            let mut end = start;
            while end < chars.len() && is_tag_char(chars[end].1) {
                end += 1;
            }
            if end > start {
                let start_byte = chars[start].0;
                let end_byte = chars
                    .get(end)
                    .map(|(byte, _)| *byte)
                    .unwrap_or(content.len());
                tags.push(content[start_byte..end_byte].to_string());
                idx = end;
                continue;
            }
        }
        idx += 1;
    }
    tags
}

fn is_tag_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

#[cfg(test)]
mod tests {
    use super::*;

    mod tempfile_free {
        use std::path::PathBuf;

        pub struct TempDir(pub PathBuf);

        impl TempDir {
            pub fn new(label: &str) -> Self {
                let mut path = std::env::temp_dir();
                let unique = format!(
                    "ramus-core-index-test-{label}-{}-{:?}",
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
    use tempfile_free::TempDir;

    #[test]
    fn extract_links_finds_basic_link() {
        assert_eq!(
            extract_links("vedi [[Progetto X]] per dettagli"),
            vec!["Progetto X"]
        );
    }

    #[test]
    fn extract_links_finds_multiple() {
        assert_eq!(
            extract_links("[[Uno]] e poi [[Due]]"),
            vec!["Uno".to_string(), "Due".to_string()]
        );
    }

    #[test]
    fn extract_links_no_match_without_brackets() {
        assert_eq!(extract_links("niente link qui"), Vec::<String>::new());
    }

    #[test]
    fn extract_links_ignores_unclosed_delimiter() {
        assert_eq!(extract_links("apre [[ma non chiude"), Vec::<String>::new());
    }

    #[test]
    fn extract_links_ignores_empty_brackets() {
        assert_eq!(extract_links("vuoto [[]] qui"), Vec::<String>::new());
    }

    #[test]
    fn extract_links_is_utf8_safe() {
        assert_eq!(extract_links("café [[città]] è qui"), vec!["città"]);
    }

    #[test]
    fn extract_tags_finds_multiple_consecutive() {
        assert_eq!(
            extract_tags("#uno#due #tre-quattro"),
            vec![
                "uno".to_string(),
                "due".to_string(),
                "tre-quattro".to_string()
            ]
        );
    }

    #[test]
    fn extract_tags_no_match_without_hash() {
        assert_eq!(extract_tags("niente tag qui"), Vec::<String>::new());
    }

    #[test]
    fn extract_tags_ignores_bare_hash() {
        assert_eq!(extract_tags("prezzo # non è un tag"), Vec::<String>::new());
    }

    #[test]
    fn extract_tags_is_utf8_safe() {
        assert_eq!(extract_tags("città #lavoro è bella"), vec!["lavoro"]);
    }

    fn open_index(dir: &Path) -> (Vault, Index) {
        let vault = Vault::new(dir.to_path_buf());
        vault.ensure_exists().unwrap();
        let index = Index::open(dir).unwrap();
        (vault, index)
    }

    #[test]
    fn sync_on_empty_vault_leaves_empty_tables() {
        let dir = TempDir::new("sync-empty-vault");
        let (vault, index) = open_index(dir.path());
        index.sync(&vault).unwrap();
        assert_eq!(index.list_tags().unwrap(), Vec::<String>::new());
        assert_eq!(index.find_backlinks("qualunque").unwrap(), Vec::new());
    }

    #[test]
    fn sync_indexes_pages_blocks_links_and_tags() {
        let dir = TempDir::new("sync-indexes");
        let (vault, index) = open_index(dir.path());
        vault.open_page("Progetto X").unwrap();
        vault
            .write_page(
                "journals/2026-01-01.md",
                &[Block::new("nota su [[Progetto X]] #lavoro")],
            )
            .unwrap();

        index.sync(&vault).unwrap();

        assert_eq!(index.list_tags().unwrap(), vec!["lavoro".to_string()]);
        let backlinks = index.find_backlinks("Progetto X").unwrap();
        assert_eq!(backlinks.len(), 1);
        assert_eq!(backlinks[0].source_path, "journals/2026-01-01.md");
        assert_eq!(backlinks[0].block_content, "nota su [[Progetto X]] #lavoro");
    }

    #[test]
    fn sync_outcome_reports_refreshed_and_removed_paths() {
        let dir = TempDir::new("sync-outcome");
        let (vault, index) = open_index(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("#uno")])
            .unwrap();
        vault
            .write_page("pages/due.md", &[Block::new("#due")])
            .unwrap();

        let first = index.sync(&vault).unwrap();
        let mut refreshed = first.refreshed.clone();
        refreshed.sort();
        assert_eq!(refreshed, vec!["pages/due.md", "pages/uno.md"]);
        assert_eq!(first.removed, Vec::<String>::new());

        let unchanged = index.sync(&vault).unwrap();
        assert_eq!(unchanged.refreshed, Vec::<String>::new());
        assert_eq!(unchanged.removed, Vec::<String>::new());

        std::fs::remove_file(dir.path().join("pages/uno.md")).unwrap();
        let after_removal = index.sync(&vault).unwrap();
        assert_eq!(after_removal.refreshed, Vec::<String>::new());
        assert_eq!(after_removal.removed, vec!["pages/uno.md".to_string()]);
    }

    #[test]
    fn sync_twice_without_changes_does_not_duplicate_rows() {
        let dir = TempDir::new("sync-twice-no-dup");
        let (vault, index) = open_index(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("#tag")])
            .unwrap();

        index.sync(&vault).unwrap();
        index.sync(&vault).unwrap();

        assert_eq!(index.list_tags().unwrap(), vec!["tag".to_string()]);
    }

    #[test]
    fn sync_after_external_deletion_removes_its_rows() {
        let dir = TempDir::new("sync-after-deletion");
        let (vault, index) = open_index(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("#tag")])
            .unwrap();
        index.sync(&vault).unwrap();
        assert_eq!(index.list_tags().unwrap(), vec!["tag".to_string()]);

        std::fs::remove_file(dir.path().join("pages/uno.md")).unwrap();
        index.sync(&vault).unwrap();
        assert_eq!(index.list_tags().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn sync_after_external_modification_refreshes_only_that_page() {
        let dir = TempDir::new("sync-after-modification");
        let (vault, index) = open_index(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("#uno")])
            .unwrap();
        vault
            .write_page("pages/due.md", &[Block::new("#due")])
            .unwrap();
        index.sync(&vault).unwrap();

        // mtime deve avanzare di almeno un secondo per essere rilevato
        // (risoluzione dell'mtime che usiamo, epoch in secondi).
        std::thread::sleep(std::time::Duration::from_millis(1100));
        vault
            .write_page("pages/uno.md", &[Block::new("#uno-modificato")])
            .unwrap();
        index.sync(&vault).unwrap();

        let mut tags = index.list_tags().unwrap();
        tags.sort();
        assert_eq!(tags, vec!["due".to_string(), "uno-modificato".to_string()]);
    }

    #[test]
    fn refresh_page_on_already_indexed_page_replaces_rows() {
        let dir = TempDir::new("refresh-replaces");
        let (vault, index) = open_index(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("#vecchio")])
            .unwrap();
        index.refresh_page(&vault, "pages/uno.md").unwrap();
        vault
            .write_page("pages/uno.md", &[Block::new("#nuovo")])
            .unwrap();
        index.refresh_page(&vault, "pages/uno.md").unwrap();

        assert_eq!(index.list_tags().unwrap(), vec!["nuovo".to_string()]);
    }

    #[test]
    fn find_backlinks_from_journal_to_page_and_back() {
        let dir = TempDir::new("backlinks-both-ways");
        let (vault, index) = open_index(dir.path());
        vault.open_page("Pagina A").unwrap();
        vault.open_page("Pagina B").unwrap();
        vault
            .write_page("journals/2026-01-01.md", &[Block::new("[[Pagina A]]")])
            .unwrap();
        vault
            .write_page("pages/pagina-b.md", &[Block::new("[[Pagina A]]")])
            .unwrap();
        index.sync(&vault).unwrap();

        let backlinks = index.find_backlinks("Pagina A").unwrap();
        let mut sources: Vec<String> = backlinks.into_iter().map(|b| b.source_path).collect();
        sources.sort();
        assert_eq!(sources, vec!["journals/2026-01-01.md", "pages/pagina-b.md"]);
    }

    #[test]
    fn find_backlinks_on_never_linked_title_is_empty_not_error() {
        let dir = TempDir::new("backlinks-never-linked");
        let (vault, index) = open_index(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("niente link qui")])
            .unwrap();
        index.sync(&vault).unwrap();
        assert_eq!(index.find_backlinks("Mai Linkata").unwrap(), Vec::new());
    }

    #[test]
    fn list_tags_deduplicates_and_sorts() {
        let dir = TempDir::new("list-tags-dedup");
        let (vault, index) = open_index(dir.path());
        vault
            .write_page(
                "pages/uno.md",
                &[Block::new("#zeta #alpha"), Block::new("#alpha")],
            )
            .unwrap();
        index.sync(&vault).unwrap();
        assert_eq!(
            index.list_tags().unwrap(),
            vec!["alpha".to_string(), "zeta".to_string()]
        );
    }

    #[test]
    fn reopening_after_deleting_index_file_rebuilds_identically() {
        let dir = TempDir::new("reopen-rebuilds");
        let (vault, index) = open_index(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("[[Altra]] #tag")])
            .unwrap();
        index.sync(&vault).unwrap();
        let before = index.list_tags().unwrap();
        drop(index);

        std::fs::remove_file(dir.path().join(".ramus/index.sqlite3")).unwrap();
        let reopened = Index::open(dir.path()).unwrap();
        reopened.sync(&vault).unwrap();
        assert_eq!(reopened.list_tags().unwrap(), before);
        assert_eq!(reopened.find_backlinks("Altra").unwrap().len(), 1);
    }
}
