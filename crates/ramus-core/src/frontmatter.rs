//! Front-matter minimale per le pagine (mai per i journal): un blocco
//! `---\n...\n---\n` in testa al file, stesso formato di Obsidian (SPEC.md,
//! "il formato su disco è compatibile con Obsidian"). Parsing scritto a
//! mano, non YAML vero e proprio: legge solo `title:`, ma preserva sempre
//! il blocco intero per il round-trip — altri campi (es. scritti da
//! Obsidian) non vengono persi, solo ignorati in lettura.

/// Se `text` inizia con `---\n...\n---\n`, separa il blocco (delimitatori
/// e newline finale inclusi) dal resto. `None` se non c'è o è malformato
/// (nessuna riga di chiusura `---`): in quel caso tutto il testo resta
/// corpo, come se non ci fosse front-matter — nessun panico.
pub fn split_front_matter(text: &str) -> (Option<&str>, &str) {
    if !text.starts_with("---\n") {
        return (None, text);
    }
    let mut offset = 4; // dopo il delimitatore di apertura "---\n"
    for line in text[4..].split_inclusive('\n') {
        if line == "---\n" || line == "---" {
            let end = offset + line.len();
            return (Some(&text[..end]), &text[end..]);
        }
        offset += line.len();
    }
    (None, text)
}

/// Legge il valore di `title:` da un front-matter grezzo (delimitatori
/// inclusi o no, non importa). Prima occorrenza, valore vuoto ignorato.
pub fn extract_title(front_matter: &str) -> Option<String> {
    for line in front_matter.lines() {
        if let Some(value) = line.strip_prefix("title:") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_well_formed_front_matter() {
        let text = "---\ntitle: Progetto X\n---\n- primo blocco\n";
        let (front, body) = split_front_matter(text);
        assert_eq!(front, Some("---\ntitle: Progetto X\n---\n"));
        assert_eq!(body, "- primo blocco\n");
    }

    #[test]
    fn round_trips_by_concatenation() {
        let text = "---\ntitle: Progetto X\n---\n- a\n  - b\n";
        let (front, body) = split_front_matter(text);
        let rejoined = format!("{}{}", front.unwrap_or(""), body);
        assert_eq!(rejoined, text);
    }

    #[test]
    fn no_front_matter_returns_whole_text_as_body() {
        let text = "- primo blocco\n";
        assert_eq!(split_front_matter(text), (None, text));
    }

    #[test]
    fn unclosed_front_matter_is_treated_as_no_front_matter() {
        let text = "---\ntitle: Progetto X\n- primo blocco\n";
        assert_eq!(split_front_matter(text), (None, text));
    }

    #[test]
    fn extracts_title() {
        let front = "---\ntitle: Progetto X\n---\n";
        assert_eq!(extract_title(front), Some("Progetto X".to_string()));
    }

    #[test]
    fn ignores_unknown_fields_without_losing_them_in_split() {
        // extract_title non li legge, ma split_front_matter preserva il
        // blocco intero: la responsabilità di non perderli è di
        // split_front_matter (verificato sopra), non di extract_title.
        let front = "---\ntags: [a, b]\ntitle: Progetto X\n---\n";
        assert_eq!(extract_title(front), Some("Progetto X".to_string()));
    }

    #[test]
    fn missing_title_field_returns_none() {
        let front = "---\ntags: [a, b]\n---\n";
        assert_eq!(extract_title(front), None);
    }

    #[test]
    fn empty_text_has_no_front_matter() {
        assert_eq!(split_front_matter(""), (None, ""));
    }
}
