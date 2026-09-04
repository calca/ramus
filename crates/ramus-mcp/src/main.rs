//! Server MCP sul vault Ramus — vedi
//! specs/M5/2026-09-02-mcp-server-lettura.DONE.md e
//! specs/M5/2026-09-02-mcp-server-scrittura.TODO.md. Binario indipendente
//! dalla GUI Tauri: stesso `ramus-core`, stesso `config.json`, nessuna
//! comunicazione diretta con il processo dell'app grafica — solo lo
//! stesso vault su disco.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ramus_core::{Config, CoreError, Index, JournalDate, SearchIndex, Vault};
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::ErrorData;
use rmcp::{tool, tool_handler, tool_router, ServerHandler, ServiceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Stesso calcolo di percorso già usato dal guscio Tauri prima di M6, ora
/// duplicato qui (non condiviso): `ramus-mcp` è desktop-only per
/// costruzione (M5, "un modello a processi che il sandboxing mobile non
/// permette"), non ha bisogno del ramo mobile via `app.path()`.
fn default_vault_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Journal")
}

fn config_file_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ramus")
        .join("config.json")
}

/// `ramus-mcp --print-config`: stampa lo snippet JSON da incollare nella
/// configurazione di un client MCP, con il percorso reale di *questo*
/// binario compilato — l'utente non deve scoprirlo/scriverlo a mano.
fn print_config() -> Result<(), serde_json::Error> {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ramus-mcp"));
    println!(
        "Aggiungi questa voce alla sezione \"mcpServers\" del file di configurazione del tuo client MCP (es. .mcp.json per Claude Code, claude_desktop_config.json per Claude Desktop):\n"
    );
    let snippet = serde_json::json!({
        "mcpServers": {
            "ramus": {
                "command": exe.to_string_lossy(),
            }
        }
    });
    println!("{}", serde_json::to_string_pretty(&snippet)?);
    println!(
        "\nPer escludere gli strumenti di scrittura (write_page, open_today, open_page), aggiungi \"args\": [\"--read-only\"] alla voce sopra."
    );
    Ok(())
}

fn core_error_to_rmcp(err: CoreError) -> ErrorData {
    ErrorData::internal_error(err.to_string(), None)
}

/// `None` se il server può avviarsi, altrimenti il messaggio da stampare
/// su stderr prima di uscire — funzione pura (non chiama
/// `std::process::exit` direttamente) per poter essere testata senza
/// terminare il processo di test.
fn mcp_disabled_message(enabled: bool) -> Option<&'static str> {
    if enabled {
        None
    } else {
        Some("ramus-mcp è disabilitato (Impostazioni → MCP in Ramus). Riabilitalo per usare questo server.")
    }
}

/// Ogni strumento restituisce il proprio risultato come stringa JSON: gli
/// stessi tipi `Serialize` già usati dai command Tauri (Page, Backlink,
/// PageSummary, SearchHit, ...), nessun tipo di risposta MCP dedicato.
fn json_result<T: Serialize>(result: Result<T, CoreError>) -> Result<String, ErrorData> {
    let value = result.map_err(core_error_to_rmcp)?;
    serde_json::to_string(&value).map_err(|e| ErrorData::internal_error(e.to_string(), None))
}

#[derive(Deserialize, JsonSchema)]
struct ListJournalsParams {
    /// Data ISO 8601 (YYYY-MM-DD): elenca i giorni di journal
    /// strettamente precedenti a questa data. Omesso per partire dal
    /// più recente.
    before: Option<String>,
    /// Numero massimo di giorni da restituire.
    limit: u32,
}

#[derive(Deserialize, JsonSchema)]
struct ReadPageParams {
    /// Path relativo al vault, es. "journals/2026-09-02.md" o
    /// "pages/nome-pagina.md".
    path: String,
}

#[derive(Deserialize, JsonSchema)]
struct SearchParams {
    /// Testo da cercare (titoli e contenuto di pagine e journal).
    query: String,
}

#[derive(Deserialize, JsonSchema)]
struct FindBacklinksParams {
    /// Titolo esatto della pagina di cui trovare i backlink.
    target_title: String,
}

/// `ramus_core::Block` non deriva `JsonSchema` (non deve — `ramus-core`
/// non dipende da `schemars`, CLAUDE.md regola 1): tipo locale a
/// `ramus-mcp`, stesso principio già seguito da `src/lib/types.ts` nel
/// frontend ("i tipi dei command rispecchiano le struct Rust, tenuti
/// allineati a mano" — qui un terzo client, stessa disciplina).
#[derive(Deserialize, JsonSchema)]
struct BlockInput {
    content: String,
    #[serde(default)]
    children: Vec<BlockInput>,
}

impl From<BlockInput> for ramus_core::Block {
    fn from(input: BlockInput) -> Self {
        ramus_core::Block {
            content: input.content,
            children: input.children.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
struct WritePageParams {
    /// Path relativo al vault della pagina o del giorno di journal da
    /// scrivere.
    path: String,
    /// Nuovo contenuto: sostituisce interamente i blocchi esistenti. Il
    /// front-matter (titolo di una pagina), se presente, viene
    /// preservato automaticamente.
    blocks: Vec<BlockInput>,
}

#[derive(Deserialize, JsonSchema)]
struct OpenPageParams {
    /// Nome della pagina (non lo slug) — creata (slug derivato dal
    /// nome) se non esiste già.
    name: String,
}

/// Processo a vita breve (avviato dal client MCP): a differenza
/// dell'app Tauri non tiene un `AppState` a lungo termine, ma `Index`/
/// `SearchIndex` restano comunque condivisi fra le chiamate di una
/// stessa sessione (costruiti una sola volta in `main`, non ad ogni
/// tool call) — `Arc<Mutex<_>>` perché i metodi `#[tool]` prendono
/// `&self`, stesso principio di `AppState` nel guscio Tauri.
#[derive(Clone)]
struct Server {
    vault_root: PathBuf,
    index: Arc<Mutex<Index>>,
    search_index: Arc<Mutex<SearchIndex>>,
    tool_router: ToolRouter<Self>,
}

impl Server {
    /// `read_only`: se `true`, gli strumenti di scrittura non entrano nel
    /// router — assenti dall'elenco esposto al client MCP, non
    /// "rifiutati a runtime" (vedi spec, `--read-only`).
    fn new(vault_root: PathBuf, index: Index, search_index: SearchIndex, read_only: bool) -> Self {
        let mut tool_router = Self::read_tool_router();
        if !read_only {
            tool_router += Self::write_tool_router();
        }
        Self {
            vault_root,
            index: Arc::new(Mutex::new(index)),
            search_index: Arc::new(Mutex::new(search_index)),
            tool_router,
        }
    }

    /// Dopo ogni scrittura: riallinea gli indici del *processo
    /// `ramus-mcp`* (quello della GUI, se aperta, si aggiorna comunque
    /// via file watcher — vedi spec, "Concorrenza con la GUI").
    fn refresh_indexes(&self, vault: &Vault, relative_path: &str) -> Result<(), ErrorData> {
        self.index
            .lock()
            .map_err(|_| ErrorData::internal_error("indice non disponibile", None))?
            .refresh_page(vault, relative_path)
            .map_err(core_error_to_rmcp)?;
        self.search_index
            .lock()
            .map_err(|_| ErrorData::internal_error("indice di ricerca non disponibile", None))?
            .refresh_page(vault, relative_path)
            .map_err(core_error_to_rmcp)?;
        Ok(())
    }
}

#[tool_router(router = read_tool_router)]
impl Server {
    #[tool(description = "Elenca i giorni di journal esistenti, più recente prima.")]
    async fn list_journals(
        &self,
        Parameters(params): Parameters<ListJournalsParams>,
    ) -> Result<String, ErrorData> {
        let before = match params.before {
            Some(text) => Some(JournalDate::parse(&text).ok_or_else(|| {
                ErrorData::invalid_params(format!("data non valida: {text}"), None)
            })?),
            None => None,
        };
        let vault = Vault::new(self.vault_root.clone());
        json_result(vault.list_journals(before, params.limit as usize))
    }

    #[tool(
        description = "Legge una pagina o un giorno di journal, dato il suo path relativo al vault."
    )]
    async fn read_page(
        &self,
        Parameters(params): Parameters<ReadPageParams>,
    ) -> Result<String, ErrorData> {
        let vault = Vault::new(self.vault_root.clone());
        json_result(vault.read_page(&params.path))
    }

    #[tool(description = "Elenca tutte le pagine del vault (slug e titolo).")]
    async fn list_pages(&self) -> Result<String, ErrorData> {
        let vault = Vault::new(self.vault_root.clone());
        json_result(vault.list_pages())
    }

    #[tool(description = "Ricerca full-text nel vault (titoli e contenuto di pagine e journal).")]
    async fn search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<String, ErrorData> {
        let search_index = self
            .search_index
            .lock()
            .map_err(|_| ErrorData::internal_error("indice di ricerca non disponibile", None))?;
        json_result(search_index.search(&params.query))
    }

    #[tool(
        description = "Trova le pagine e i giorni di journal che contengono [[il titolo indicato]]."
    )]
    async fn find_backlinks(
        &self,
        Parameters(params): Parameters<FindBacklinksParams>,
    ) -> Result<String, ErrorData> {
        let index = self
            .index
            .lock()
            .map_err(|_| ErrorData::internal_error("indice non disponibile", None))?;
        json_result(index.find_backlinks(&params.target_title))
    }

    #[tool(description = "Elenca tutti i #tag usati nel vault.")]
    async fn list_tags(&self) -> Result<String, ErrorData> {
        let index = self
            .index
            .lock()
            .map_err(|_| ErrorData::internal_error("indice non disponibile", None))?;
        json_result(index.list_tags())
    }

    #[tool(
        description = "Elenca tutti i task \"[ ] \" aperti (non fatti) nel vault, in qualunque journal o pagina."
    )]
    async fn list_open_tasks(&self) -> Result<String, ErrorData> {
        let index = self
            .index
            .lock()
            .map_err(|_| ErrorData::internal_error("indice non disponibile", None))?;
        json_result(index.list_open_tasks())
    }
}

#[tool_router(router = write_tool_router)]
impl Server {
    #[tool(
        description = "Sovrascrive i blocchi di una pagina o giorno di journal esistente. Il front-matter (titolo di una pagina), se presente, viene preservato automaticamente."
    )]
    async fn write_page(
        &self,
        Parameters(params): Parameters<WritePageParams>,
    ) -> Result<String, ErrorData> {
        let vault = Vault::new(self.vault_root.clone());
        let blocks: Vec<ramus_core::Block> = params.blocks.into_iter().map(Into::into).collect();
        vault
            .write_page(&params.path, &blocks)
            .map_err(core_error_to_rmcp)?;
        self.refresh_indexes(&vault, &params.path)?;
        json_result(vault.read_page(&params.path))
    }

    #[tool(
        description = "Apre il journal di oggi, creandolo con un blocco vuoto se non esiste ancora."
    )]
    async fn open_today(&self) -> Result<String, ErrorData> {
        let vault = Vault::new(self.vault_root.clone());
        let page = vault.open_today().map_err(core_error_to_rmcp)?;
        let relative_path = page.path.to_string_lossy().to_string();
        self.refresh_indexes(&vault, &relative_path)?;
        serde_json::to_string(&page).map_err(|e| ErrorData::internal_error(e.to_string(), None))
    }

    #[tool(
        description = "Apre la pagina indicata per nome, creandola (slug derivato dal nome) se non esiste già."
    )]
    async fn open_page(
        &self,
        Parameters(params): Parameters<OpenPageParams>,
    ) -> Result<String, ErrorData> {
        let vault = Vault::new(self.vault_root.clone());
        let page = vault.open_page(&params.name).map_err(core_error_to_rmcp)?;
        let relative_path = page.path.to_string_lossy().to_string();
        self.refresh_indexes(&vault, &relative_path)?;
        serde_json::to_string(&page).map_err(|e| ErrorData::internal_error(e.to_string(), None))
    }
}

#[tool_handler(
    router = self.tool_router,
    name = "ramus-mcp",
    instructions = "Strumenti sul vault di Ramus (journal e pagine markdown). Lettura: list_journals, read_page, list_pages, search, find_backlinks, list_tags, list_open_tasks. Scrittura (assenti se avviato con --read-only): write_page, open_today, open_page."
)]
impl ServerHandler for Server {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--print-config") {
        print_config()?;
        return Ok(());
    }
    let read_only = args.iter().any(|arg| arg == "--read-only");

    let config = Config::load_or_init(&config_file_path(), default_vault_path())?;
    if let Some(message) = mcp_disabled_message(config.mcp_enabled) {
        eprintln!("{message}");
        std::process::exit(1);
    }

    let vault_root = config.vault_path.clone();
    let vault = Vault::new(vault_root.clone());
    vault.ensure_exists()?;

    // Stessa sequenza di src-tauri/src/lib.rs::run — copiata non
    // condivisa, non vale un'astrazione per due chiamate identiche in
    // due binari diversi (vedi spec, "Avvio: indici sempre freschi").
    let index = Index::open(&vault_root)?;
    let outcome = index.sync(&vault)?;

    let search_index = SearchIndex::open(&vault_root)?;
    for path in &outcome.refreshed {
        search_index.refresh_page(&vault, path)?;
    }
    for path in &outcome.removed {
        search_index.remove_page(path)?;
    }

    let server = Server::new(vault_root, index, search_index, read_only);
    let service = server.serve(rmcp::transport::stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ramus_core::Block;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let mut path = std::env::temp_dir();
            let unique = format!(
                "ramus-mcp-test-{label}-{}-{:?}",
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
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Stessa sequenza di `main()` (Index::sync + refresh del
    /// SearchIndex), su un vault di prova — non tramite il transport
    /// MCP, chiamando direttamente i metodi degli strumenti (stesso
    /// principio già usato per i command Tauri equivalenti).
    fn test_server(dir: &TempDir) -> Server {
        test_server_with(dir, false)
    }

    fn test_server_with(dir: &TempDir, read_only: bool) -> Server {
        let vault_root = dir.0.clone();
        let vault = Vault::new(vault_root.clone());
        vault.ensure_exists().unwrap();

        let index = Index::open(&vault_root).unwrap();
        let outcome = index.sync(&vault).unwrap();

        let search_index = SearchIndex::open(&vault_root).unwrap();
        for path in &outcome.refreshed {
            search_index.refresh_page(&vault, path).unwrap();
        }

        Server::new(vault_root, index, search_index, read_only)
    }

    #[tokio::test]
    async fn list_pages_returns_pages_written_to_the_vault() {
        let dir = TempDir::new("list-pages");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        vault.open_page("Progetto Alfa").unwrap();

        let server = test_server(&dir);
        let json = server.list_pages().await.unwrap();
        assert!(json.contains("Progetto Alfa"), "risposta: {json}");
    }

    #[tokio::test]
    async fn list_journals_respects_limit() {
        let dir = TempDir::new("list-journals");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        vault
            .write_page("journals/2026-01-01.md", &[Block::new("uno")])
            .unwrap();
        vault
            .write_page("journals/2026-01-02.md", &[Block::new("due")])
            .unwrap();
        vault
            .write_page("journals/2026-01-03.md", &[Block::new("tre")])
            .unwrap();

        let server = test_server(&dir);
        let json = server
            .list_journals(Parameters(ListJournalsParams {
                before: None,
                limit: 2,
            }))
            .await
            .unwrap();
        let parsed: Vec<ramus_core::Page> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].path, PathBuf::from("journals/2026-01-03.md"));
    }

    #[tokio::test]
    async fn list_journals_rejects_invalid_before_date() {
        let dir = TempDir::new("invalid-date");
        let server = test_server(&dir);
        let result = server
            .list_journals(Parameters(ListJournalsParams {
                before: Some("not-a-date".to_string()),
                limit: 10,
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_page_returns_the_written_content() {
        let dir = TempDir::new("read-page");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        vault
            .write_page("pages/nota.md", &[Block::new("contenuto di prova")])
            .unwrap();

        let server = test_server(&dir);
        let json = server
            .read_page(Parameters(ReadPageParams {
                path: "pages/nota.md".to_string(),
            }))
            .await
            .unwrap();
        assert!(json.contains("contenuto di prova"), "risposta: {json}");
    }

    #[tokio::test]
    async fn read_page_missing_returns_an_error_not_a_panic() {
        let dir = TempDir::new("read-missing");
        let server = test_server(&dir);
        let result = server
            .read_page(Parameters(ReadPageParams {
                path: "pages/non-esiste.md".to_string(),
            }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn search_finds_content_indexed_at_startup() {
        let dir = TempDir::new("search");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        vault
            .write_page(
                "pages/nota.md",
                &[Block::new("parola rarissima da cercare")],
            )
            .unwrap();

        let server = test_server(&dir);
        let json = server
            .search(Parameters(SearchParams {
                query: "rarissima".to_string(),
            }))
            .await
            .unwrap();
        assert!(json.contains("nota.md"), "risposta: {json}");
    }

    #[tokio::test]
    async fn find_backlinks_finds_a_link_to_the_target_title() {
        let dir = TempDir::new("backlinks");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        vault.open_page("Pagina Bersaglio").unwrap();
        vault
            .write_page("pages/altra.md", &[Block::new("vedi [[Pagina Bersaglio]]")])
            .unwrap();

        let server = test_server(&dir);
        let json = server
            .find_backlinks(Parameters(FindBacklinksParams {
                target_title: "Pagina Bersaglio".to_string(),
            }))
            .await
            .unwrap();
        assert!(json.contains("altra.md"), "risposta: {json}");
    }

    #[tokio::test]
    async fn list_tags_lists_tags_used_in_the_vault() {
        let dir = TempDir::new("tags");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        vault
            .write_page("pages/nota.md", &[Block::new("un blocco con #ramus")])
            .unwrap();

        let server = test_server(&dir);
        let json = server.list_tags().await.unwrap();
        assert!(json.contains("ramus"), "risposta: {json}");
    }

    #[tokio::test]
    async fn list_open_tasks_lists_only_the_open_task() {
        let dir = TempDir::new("open-tasks");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        vault
            .write_page(
                "pages/nota.md",
                &[Block::new("[ ] task aperto"), Block::new("[x] task fatto")],
            )
            .unwrap();

        let server = test_server(&dir);
        let json = server.list_open_tasks().await.unwrap();
        assert!(json.contains("task aperto"), "risposta: {json}");
        assert!(!json.contains("task fatto"), "risposta: {json}");
    }

    #[tokio::test]
    async fn write_page_produces_the_same_file_as_a_direct_vault_write() {
        let dir = TempDir::new("write-page");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();

        let server = test_server(&dir);
        server
            .write_page(Parameters(WritePageParams {
                path: "pages/nota.md".to_string(),
                blocks: vec![BlockInput {
                    content: "scritto da mcp".to_string(),
                    children: vec![],
                }],
            }))
            .await
            .unwrap();

        let page = vault.read_page("pages/nota.md").unwrap();
        assert_eq!(page.blocks[0].content, "scritto da mcp");
    }

    #[tokio::test]
    async fn write_page_then_search_in_the_same_session_finds_new_content() {
        let dir = TempDir::new("write-then-search");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();

        let server = test_server(&dir);
        server
            .write_page(Parameters(WritePageParams {
                path: "pages/nota.md".to_string(),
                blocks: vec![BlockInput {
                    content: "parola specialissima".to_string(),
                    children: vec![],
                }],
            }))
            .await
            .unwrap();

        let json = server
            .search(Parameters(SearchParams {
                query: "specialissima".to_string(),
            }))
            .await
            .unwrap();
        assert!(json.contains("nota.md"), "risposta: {json}");
    }

    #[tokio::test]
    async fn open_today_creates_the_journal_file_if_missing() {
        let dir = TempDir::new("open-today");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();

        let server = test_server(&dir);
        let json = server.open_today().await.unwrap();
        let page: ramus_core::Page = serde_json::from_str(&json).unwrap();
        assert!(vault
            .resolve(&page.path.to_string_lossy())
            .unwrap()
            .exists());
    }

    #[tokio::test]
    async fn open_page_creates_a_page_with_the_given_title() {
        let dir = TempDir::new("open-page");
        let server = test_server(&dir);
        let json = server
            .open_page(Parameters(OpenPageParams {
                name: "Nuova Pagina".to_string(),
            }))
            .await
            .unwrap();
        assert!(json.contains("Nuova Pagina"), "risposta: {json}");
    }

    #[test]
    fn read_only_excludes_write_tools_from_the_router() {
        let dir = TempDir::new("read-only");
        let server = test_server_with(&dir, true);
        assert!(!server.tool_router.has_route("write_page"));
        assert!(!server.tool_router.has_route("open_today"));
        assert!(!server.tool_router.has_route("open_page"));
        assert!(server.tool_router.has_route("search"));
    }

    #[test]
    fn writable_by_default_includes_write_tools() {
        let dir = TempDir::new("writable-default");
        let server = test_server(&dir);
        assert!(server.tool_router.has_route("write_page"));
        assert!(server.tool_router.has_route("open_today"));
        assert!(server.tool_router.has_route("open_page"));
    }

    #[test]
    fn mcp_disabled_message_is_none_when_enabled_and_some_when_disabled() {
        assert!(mcp_disabled_message(true).is_none());
        assert!(mcp_disabled_message(false).is_some());
    }
}
