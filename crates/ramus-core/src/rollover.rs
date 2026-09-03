//! Sposta automaticamente i task `[ ] ` non fatti rimasti nei giorni
//! passati del journal verso il giorno di oggi — vedi
//! specs/M4/2026-09-02-task-todo-done.TODO.md, sezione "Spostare un task
//! a oggi" (rivista: automatico su una finestra di giorni configurabile,
//! non un bottone manuale per singolo task).

use serde::{Deserialize, Serialize};

use crate::block::Block;
use crate::error::CoreError;
use crate::journal_date::JournalDate;
use crate::vault::Vault;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RolloverOutcome {
    pub moved_count: usize,
}

fn is_unfinished_task(content: &str) -> bool {
    content.starts_with("[ ] ")
}

/// Estrae ricorsivamente dai blocchi ogni sottoalbero la cui radice è un
/// task non fatto, lasciando intatto il resto della struttura. I figli di
/// un sottoalbero estratto non vengono scansionati a loro volta (vengono
/// con il genitore, non separatamente).
fn extract_unfinished_tasks(blocks: Vec<Block>) -> (Vec<Block>, Vec<Block>) {
    let mut remaining = Vec::with_capacity(blocks.len());
    let mut extracted = Vec::new();
    for mut block in blocks {
        if is_unfinished_task(&block.content) {
            extracted.push(block);
        } else {
            let (children_remaining, mut children_extracted) =
                extract_unfinished_tasks(block.children);
            block.children = children_remaining;
            remaining.push(block);
            extracted.append(&mut children_extracted);
        }
    }
    (remaining, extracted)
}

/// Scansiona i `days_back` giorni di journal precedenti a oggi (i più
/// vecchi per primi), estrae ogni task `[ ] ` non fatto (sottoalbero
/// incluso) e lo aggiunge in fondo al journal di oggi. Ordine che evita
/// di perdere dati: si scrive prima la destinazione (oggi) con tutti i
/// task raccolti — se fallisce, nessuna sorgente è stata toccata, i task
/// restano dov'erano — solo dopo un salvataggio riuscito si riscrivono le
/// sorgenti senza i task spostati.
pub fn roll_over_unfinished_tasks(
    vault: &Vault,
    days_back: u32,
) -> Result<RolloverOutcome, CoreError> {
    let mut dates = Vec::with_capacity(days_back as usize);
    let mut date = JournalDate::today();
    for _ in 0..days_back {
        date = date.pred();
        dates.push(date);
    }
    dates.reverse(); // dal più vecchio al più recente

    let mut moved: Vec<Block> = Vec::new();
    let mut pruned_sources: Vec<(String, Vec<Block>)> = Vec::new();

    for date in dates {
        let relative_path = Vault::journal_relative_path(date);
        let abs = vault.resolve(&relative_path)?;
        if !abs.exists() {
            continue;
        }
        let page = vault.read_page(&relative_path)?;
        let (remaining, mut extracted) = extract_unfinished_tasks(page.blocks);
        if extracted.is_empty() {
            continue;
        }
        moved.append(&mut extracted);
        pruned_sources.push((relative_path, remaining));
    }

    let moved_count = moved.len();
    if moved_count == 0 {
        return Ok(RolloverOutcome { moved_count: 0 });
    }

    let mut today_page = vault.open_today()?;
    today_page.blocks.append(&mut moved);
    let today_relative = Vault::journal_relative_path(JournalDate::today());
    vault.write_page(&today_relative, &today_page.blocks)?;

    for (relative_path, remaining) in &pruned_sources {
        vault.write_page(relative_path, remaining)?;
    }

    Ok(RolloverOutcome { moved_count })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let mut path = std::env::temp_dir();
            let unique = format!(
                "ramus-core-rollover-test-{label}-{}-{:?}",
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

    fn journal_path(dir: &TempDir, date: JournalDate) -> PathBuf {
        dir.0.join(Vault::journal_relative_path(date))
    }

    fn write_journal(dir: &TempDir, date: JournalDate, content: &str) {
        let path = journal_path(dir, date);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn read_journal(dir: &TempDir, date: JournalDate) -> String {
        std::fs::read_to_string(journal_path(dir, date)).unwrap()
    }

    #[test]
    fn moves_unfinished_task_from_yesterday_to_today() {
        let dir = TempDir::new("basic-move");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        let yesterday = JournalDate::today().pred();
        write_journal(&dir, yesterday, "- [ ] Comprare il latte\n- Nota normale\n");

        let outcome = roll_over_unfinished_tasks(&vault, 7).unwrap();
        assert_eq!(outcome.moved_count, 1);

        let today_content = read_journal(&dir, JournalDate::today());
        assert!(today_content.contains("[ ] Comprare il latte"));

        let yesterday_content = read_journal(&dir, yesterday);
        assert!(!yesterday_content.contains("[ ] Comprare il latte"));
        assert!(yesterday_content.contains("Nota normale"));
    }

    #[test]
    fn leaves_done_tasks_and_normal_blocks_in_place() {
        let dir = TempDir::new("leaves-done");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        let yesterday = JournalDate::today().pred();
        write_journal(&dir, yesterday, "- [x] Fatto\n- Nota normale\n");

        let outcome = roll_over_unfinished_tasks(&vault, 7).unwrap();
        assert_eq!(outcome.moved_count, 0);

        let yesterday_content = read_journal(&dir, yesterday);
        assert!(yesterday_content.contains("[x] Fatto"));
        assert!(yesterday_content.contains("Nota normale"));
    }

    #[test]
    fn moves_subtree_together_with_its_task_root() {
        let dir = TempDir::new("subtree");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        let yesterday = JournalDate::today().pred();
        write_journal(
            &dir,
            yesterday,
            "- [ ] Preparare la presentazione\n  - Slide 1\n  - Slide 2\n",
        );

        roll_over_unfinished_tasks(&vault, 7).unwrap();

        let today_content = read_journal(&dir, JournalDate::today());
        assert!(today_content.contains("[ ] Preparare la presentazione"));
        assert!(today_content.contains("Slide 1"));
        assert!(today_content.contains("Slide 2"));
    }

    #[test]
    fn finds_task_nested_under_a_non_task_block() {
        let dir = TempDir::new("nested-task");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        let yesterday = JournalDate::today().pred();
        write_journal(
            &dir,
            yesterday,
            "- Riunione con il cliente\n  - [ ] Mandare il preventivo\n",
        );

        let outcome = roll_over_unfinished_tasks(&vault, 7).unwrap();
        assert_eq!(outcome.moved_count, 1);

        let yesterday_content = read_journal(&dir, yesterday);
        assert!(yesterday_content.contains("Riunione con il cliente"));
        assert!(!yesterday_content.contains("[ ] Mandare il preventivo"));

        let today_content = read_journal(&dir, JournalDate::today());
        assert!(today_content.contains("[ ] Mandare il preventivo"));
    }

    #[test]
    fn ignores_days_outside_the_lookback_window() {
        let dir = TempDir::new("outside-window");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        let mut too_old = JournalDate::today();
        for _ in 0..10 {
            too_old = too_old.pred();
        }
        write_journal(&dir, too_old, "- [ ] Task troppo vecchio\n");

        let outcome = roll_over_unfinished_tasks(&vault, 7).unwrap();
        assert_eq!(outcome.moved_count, 0);

        let content = read_journal(&dir, too_old);
        assert!(content.contains("[ ] Task troppo vecchio"));
    }

    #[test]
    fn no_op_on_a_vault_with_no_unfinished_tasks_writes_nothing() {
        let dir = TempDir::new("no-op");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        let yesterday = JournalDate::today().pred();
        write_journal(&dir, yesterday, "- Solo una nota\n");

        let outcome = roll_over_unfinished_tasks(&vault, 7).unwrap();
        assert_eq!(outcome.moved_count, 0);

        // Nessun file "oggi" creato: zero task da spostare, zero scritture.
        assert!(!journal_path(&dir, JournalDate::today()).exists());
    }

    #[test]
    fn collects_tasks_from_multiple_days_oldest_first() {
        let dir = TempDir::new("multiple-days");
        let vault = Vault::new(dir.0.clone());
        vault.ensure_exists().unwrap();
        let yesterday = JournalDate::today().pred();
        let two_days_ago = yesterday.pred();
        write_journal(&dir, two_days_ago, "- [ ] Il più vecchio\n");
        write_journal(&dir, yesterday, "- [ ] Il più recente\n");

        let outcome = roll_over_unfinished_tasks(&vault, 7).unwrap();
        assert_eq!(outcome.moved_count, 2);

        let today_content = read_journal(&dir, JournalDate::today());
        let pos_oldest = today_content.find("Il più vecchio").unwrap();
        let pos_recent = today_content.find("Il più recente").unwrap();
        assert!(pos_oldest < pos_recent);
    }
}
