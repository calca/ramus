//! Indice full-text (tantivy) di pagine/journal, per pagina intera — vedi
//! specs/2026-09-02-ricerca-full-text.md. "Dumb": non tiene una propria
//! contabilità di mtime, viene guidato dal diff già calcolato da
//! `Index::sync` (`SyncOutcome`, in `index.rs`) e dagli stessi punti di
//! chiamata già usati per `Index::refresh_page`.

use std::fs;
use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, Value, STORED, STRING, TEXT};
use tantivy::snippet::SnippetGenerator;
use tantivy::{
    doc, Index as TantivyIndex, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term,
};

use crate::error::CoreError;
use crate::index::flatten_blocks;
use crate::vault::Vault;

const MAX_SEARCH_RESULTS: usize = 20;
const SNIPPET_MAX_CHARS: usize = 160;
const WRITER_HEAP_BYTES: usize = 25_000_000;

/// Un risultato di ricerca: una pagina o un giorno di journal intero, mai
/// un singolo blocco (vedi "Granularità" nella spec).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SearchHit {
    pub path: String,
    pub kind: String,
    pub title: Option<String>,
    /// HTML generato da tantivy (`Snippet::to_html()`): testo circostante
    /// fuggito, termini che combaciano avvolti in `<b>`. Sicuro da rendere
    /// con `dangerouslySetInnerHTML` lato frontend — l'unico markup
    /// iniettato è quello che genera tantivy stesso.
    pub snippet_html: String,
}

pub struct SearchIndex {
    index: TantivyIndex,
    reader: IndexReader,
    path_field: Field,
    kind_field: Field,
    title_field: Field,
    content_field: Field,
}

impl SearchIndex {
    /// Apre (creando se serve) `<vault_root>/.ramus/search-index/`.
    pub fn open(vault_root: &Path) -> Result<Self, CoreError> {
        let dir = vault_root.join(".ramus").join("search-index");
        fs::create_dir_all(&dir).map_err(|source| CoreError::Io {
            path: dir.clone(),
            source,
        })?;

        let mut schema_builder = Schema::builder();
        // STRING (non tokenizzato) per path/kind: serve solo un match esatto
        // per delete_term, mai una ricerca full-text su questi campi.
        let path_field = schema_builder.add_text_field("path", STRING | STORED);
        let kind_field = schema_builder.add_text_field("kind", STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let schema = schema_builder.build();

        let index = if dir.join("meta.json").exists() {
            TantivyIndex::open_in_dir(&dir)?
        } else {
            TantivyIndex::create_in_dir(&dir, schema)?
        };

        // ReloadPolicy::Manual + reload() esplicito a inizio ricerca (vedi
        // `search`): il default (OnCommitWithDelay) ricarica il reader in
        // background dopo un commit, senza garanzia di immediatezza — per
        // un'app mono-utente a bassa frequenza di ricerca, la semplicità di
        // "sempre fresco" vale il piccolo overhead per ricerca.
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            path_field,
            kind_field,
            title_field,
            content_field,
        })
    }

    /// Rilegge una pagina dal vault e sostituisce il suo documento
    /// nell'indice (delete_term su `path` + reinserimento + commit).
    pub fn refresh_page(&self, vault: &Vault, relative_path: &str) -> Result<(), CoreError> {
        let page = vault.read_page(relative_path)?;
        let kind = if relative_path.starts_with("journals/") {
            "journal"
        } else {
            "page"
        };
        let content = flatten_blocks(&page.blocks)
            .into_iter()
            .map(|block| block.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let title = page.title.unwrap_or_default();

        let mut writer: IndexWriter = self.index.writer(WRITER_HEAP_BYTES)?;
        writer.delete_term(Term::from_field_text(self.path_field, relative_path));
        writer.add_document(doc!(
            self.path_field => relative_path,
            self.kind_field => kind,
            self.title_field => title,
            self.content_field => content,
        ))?;
        writer.commit()?;
        Ok(())
    }

    /// Rimuove il documento di una pagina non più presente su disco.
    pub fn remove_page(&self, relative_path: &str) -> Result<(), CoreError> {
        let mut writer: IndexWriter = self.index.writer(WRITER_HEAP_BYTES)?;
        writer.delete_term(Term::from_field_text(self.path_field, relative_path));
        writer.commit()?;
        Ok(())
    }

    /// Cerca su `title` + `content`, fino a `MAX_SEARCH_RESULTS` risultati
    /// ordinati per rilevanza. Query vuota o non interpretabile (es.
    /// virgolette sbilanciate) → lista vuota, non errore: l'utente sta
    /// ancora digitando, non è un caso da trattare come fallimento.
    pub fn search(&self, query: &str) -> Result<Vec<SearchHit>, CoreError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        self.reader.reload()?;
        let searcher = self.reader.searcher();
        let query_parser =
            QueryParser::for_index(&self.index, vec![self.title_field, self.content_field]);
        let Ok(parsed) = query_parser.parse_query(trimmed) else {
            return Ok(Vec::new());
        };

        let top_docs = searcher.search(
            &parsed,
            &TopDocs::with_limit(MAX_SEARCH_RESULTS).order_by_score(),
        )?;

        let mut snippet_generator =
            SnippetGenerator::create(&searcher, &parsed, self.content_field)?;
        snippet_generator.set_max_num_chars(SNIPPET_MAX_CHARS);

        let mut hits = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved: TantivyDocument = searcher.doc(doc_address)?;
            let path = field_text(&retrieved, self.path_field).unwrap_or_default();
            let kind = field_text(&retrieved, self.kind_field).unwrap_or_default();
            let title = field_text(&retrieved, self.title_field).filter(|t| !t.is_empty());
            let snippet = snippet_generator.snippet_from_doc(&retrieved);
            hits.push(SearchHit {
                path,
                kind,
                title,
                snippet_html: snippet.to_html(),
            });
        }
        Ok(hits)
    }
}

fn field_text(doc: &TantivyDocument, field: Field) -> Option<String> {
    doc.get_first(field)
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Block;

    mod tempfile_free {
        use std::path::PathBuf;

        pub struct TempDir(pub PathBuf);

        impl TempDir {
            pub fn new(label: &str) -> Self {
                let mut path = std::env::temp_dir();
                let unique = format!(
                    "ramus-core-search-test-{label}-{}-{:?}",
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

    fn open(dir: &Path) -> (Vault, SearchIndex) {
        let vault = Vault::new(dir.to_path_buf());
        vault.ensure_exists().unwrap();
        let search_index = SearchIndex::open(dir).unwrap();
        (vault, search_index)
    }

    #[test]
    fn open_on_empty_directory_does_not_fail() {
        let dir = TempDir::new("open-empty");
        let (_, search_index) = open(dir.path());
        assert_eq!(search_index.search("qualunque").unwrap(), Vec::new());
    }

    #[test]
    fn refresh_page_makes_content_findable() {
        let dir = TempDir::new("refresh-findable");
        let (vault, search_index) = open(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("una nota su elefanti rosa")])
            .unwrap();
        search_index.refresh_page(&vault, "pages/uno.md").unwrap();

        let hits = search_index.search("elefanti").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "pages/uno.md");
        assert_eq!(hits[0].kind, "page");
        assert!(hits[0].snippet_html.contains("<b>elefanti</b>"));
    }

    #[test]
    fn search_with_absent_term_is_empty_not_error() {
        let dir = TempDir::new("search-absent");
        let (vault, search_index) = open(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("contenuto qualsiasi")])
            .unwrap();
        search_index.refresh_page(&vault, "pages/uno.md").unwrap();

        assert_eq!(search_index.search("introvabile").unwrap(), Vec::new());
    }

    #[test]
    fn search_with_empty_query_is_empty_without_parsing() {
        let dir = TempDir::new("search-empty-query");
        let (_, search_index) = open(dir.path());
        assert_eq!(search_index.search("").unwrap(), Vec::new());
        assert_eq!(search_index.search("   ").unwrap(), Vec::new());
    }

    #[test]
    fn search_with_malformed_query_is_empty_not_error() {
        let dir = TempDir::new("search-malformed-query");
        let (vault, search_index) = open(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("contenuto")])
            .unwrap();
        search_index.refresh_page(&vault, "pages/uno.md").unwrap();

        assert_eq!(
            search_index.search("\"virgolette sbilanciate").unwrap(),
            Vec::new()
        );
    }

    #[test]
    fn remove_page_makes_it_unfindable() {
        let dir = TempDir::new("remove-page");
        let (vault, search_index) = open(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("contenuto cercabile")])
            .unwrap();
        search_index.refresh_page(&vault, "pages/uno.md").unwrap();
        assert_eq!(search_index.search("cercabile").unwrap().len(), 1);

        search_index.remove_page("pages/uno.md").unwrap();
        assert_eq!(search_index.search("cercabile").unwrap(), Vec::new());
    }

    #[test]
    fn refresh_page_twice_does_not_duplicate_results() {
        let dir = TempDir::new("refresh-twice");
        let (vault, search_index) = open(dir.path());
        vault
            .write_page("pages/uno.md", &[Block::new("prima versione")])
            .unwrap();
        search_index.refresh_page(&vault, "pages/uno.md").unwrap();
        vault
            .write_page("pages/uno.md", &[Block::new("seconda versione")])
            .unwrap();
        search_index.refresh_page(&vault, "pages/uno.md").unwrap();

        assert_eq!(search_index.search("versione").unwrap().len(), 1);
        assert_eq!(search_index.search("prima").unwrap(), Vec::new());
    }

    #[test]
    fn journal_hit_reports_journal_kind() {
        let dir = TempDir::new("journal-kind");
        let (vault, search_index) = open(dir.path());
        vault
            .write_page("journals/2026-01-01.md", &[Block::new("giornata intensa")])
            .unwrap();
        search_index
            .refresh_page(&vault, "journals/2026-01-01.md")
            .unwrap();

        let hits = search_index.search("intensa").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, "journal");
        assert_eq!(hits[0].title, None);
    }
}
