//! Conversione fra il formato su disco (righe `- ` indentate a due spazi per
//! livello) e l'albero di [`Block`]. Deve garantire il round-trip esatto:
//! `parse(render(blocks)) == blocks` per qualunque albero, e
//! `render(parse(text)) == text` per file già conformi al formato.

use crate::block::Block;
use crate::error::CoreError;

/// Una singola riga del file, già scomposta in livello di indentazione e
/// contenuto (senza indentazione né prefisso "- ").
struct Line {
    level: usize,
    content: String,
}

fn parse_line(raw: &str, index: usize) -> Result<Line, CoreError> {
    let indent_len = raw.len() - raw.trim_start_matches(' ').len();
    let (indent, rest) = raw.split_at(indent_len);

    if indent.contains('\t') {
        return Err(CoreError::MalformedBlock {
            line: index,
            reason: "l'indentazione usa tab, non spazi".to_string(),
        });
    }
    if !indent_len.is_multiple_of(2) {
        return Err(CoreError::MalformedBlock {
            line: index,
            reason: "l'indentazione non è un multiplo di due spazi".to_string(),
        });
    }

    let content = if let Some(content) = rest.strip_prefix("- ") {
        content.to_string()
    } else if rest == "-" {
        String::new()
    } else {
        return Err(CoreError::MalformedBlock {
            line: index,
            reason: format!("la riga non inizia con \"- \": {rest:?}"),
        });
    };

    Ok(Line {
        level: indent_len / 2,
        content,
    })
}

/// Converte il testo di un file di pagina nell'albero di blocchi.
pub fn parse(text: &str) -> Result<Vec<Block>, CoreError> {
    if text.is_empty() {
        return Ok(Vec::new());
    }

    let mut raw_lines: Vec<&str> = text.split('\n').collect();
    // Il file termina sempre con un newline finale: l'ultimo elemento dello
    // split è una stringa vuota, non una riga reale.
    if raw_lines.last() == Some(&"") {
        raw_lines.pop();
    }

    let mut lines = Vec::with_capacity(raw_lines.len());
    for (i, raw) in raw_lines.iter().enumerate() {
        lines.push(parse_line(raw, i + 1)?);
    }

    if lines[0].level != 0 {
        return Err(CoreError::MalformedBlock {
            line: 1,
            reason: "il primo blocco non può essere indentato".to_string(),
        });
    }
    for window in lines.windows(2) {
        if window[1].level > window[0].level + 1 {
            return Err(CoreError::MalformedBlock {
                line: 0,
                reason: "salto di più di un livello di indentazione".to_string(),
            });
        }
    }

    let mut idx = 0;
    Ok(build_tree(&lines, &mut idx, 0))
}

fn build_tree(lines: &[Line], idx: &mut usize, level: usize) -> Vec<Block> {
    let mut blocks = Vec::new();
    while *idx < lines.len() {
        if lines[*idx].level < level {
            break;
        }
        let content = lines[*idx].content.clone();
        *idx += 1;
        let children = build_tree(lines, idx, level + 1);
        blocks.push(Block { content, children });
    }
    blocks
}

/// Converte l'albero di blocchi nel testo del file, con newline finale.
pub fn render(blocks: &[Block]) -> String {
    let mut out = String::new();
    render_into(blocks, 0, &mut out);
    out
}

fn render_into(blocks: &[Block], level: usize, out: &mut String) {
    for block in blocks {
        for _ in 0..level {
            out.push_str("  ");
        }
        out.push_str("- ");
        out.push_str(&block.content);
        out.push('\n');
        render_into(&block.children, level + 1, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_blocks() -> Vec<Block> {
        vec![
            Block {
                content: "Riunione con il cliente".to_string(),
                children: vec![
                    Block::new("Deciso di rimandare il rilascio"),
                    Block::new("Da verificare: capacità del team a ottobre"),
                ],
            },
            Block::new("Nota personale"),
        ]
    }

    #[test]
    fn render_matches_spec_example() {
        let expected = "- Riunione con il cliente\n\
                         \x20\x20- Deciso di rimandare il rilascio\n\
                         \x20\x20- Da verificare: capacità del team a ottobre\n\
                         - Nota personale\n";
        assert_eq!(render(&sample_blocks()), expected);
    }

    #[test]
    fn round_trip_parse_render() {
        let blocks = sample_blocks();
        let text = render(&blocks);
        assert_eq!(parse(&text).unwrap(), blocks);
    }

    #[test]
    fn round_trip_render_parse_is_identity_on_conformant_text() {
        let text = "- a\n  - b\n    - c\n  - d\n- e\n";
        assert_eq!(render(&parse(text).unwrap()), text);
    }

    #[test]
    fn empty_page_round_trips() {
        assert_eq!(parse("").unwrap(), Vec::<Block>::new());
        assert_eq!(render(&[]), "");
    }

    #[test]
    fn single_empty_block_round_trips() {
        let blocks = vec![Block::new("")];
        let text = render(&blocks);
        assert_eq!(text, "- \n");
        assert_eq!(parse(&text).unwrap(), blocks);
    }

    #[test]
    fn dash_without_trailing_space_is_empty_content() {
        // Alcuni editor tolgono lo spazio finale su riga vuota: deve restare leggibile.
        assert_eq!(parse("-\n").unwrap(), vec![Block::new("")]);
    }

    #[test]
    fn rejects_odd_indentation() {
        let err = parse(" - a\n").unwrap_err();
        assert!(matches!(err, CoreError::MalformedBlock { .. }));
    }

    #[test]
    fn rejects_tab_indentation() {
        let err = parse("\t- a\n").unwrap_err();
        assert!(matches!(err, CoreError::MalformedBlock { .. }));
    }

    #[test]
    fn rejects_first_block_indented() {
        let err = parse("  - a\n").unwrap_err();
        assert!(matches!(err, CoreError::MalformedBlock { .. }));
    }

    #[test]
    fn rejects_indentation_jump() {
        let err = parse("- a\n    - b\n").unwrap_err();
        assert!(matches!(err, CoreError::MalformedBlock { .. }));
    }

    #[test]
    fn rejects_line_without_dash() {
        let err = parse("just text\n").unwrap_err();
        assert!(matches!(err, CoreError::MalformedBlock { .. }));
    }

    #[test]
    fn deep_nesting_round_trips() {
        let mut blocks = vec![Block::new("root")];
        let mut cursor = &mut blocks[0];
        for i in 0..20 {
            cursor.children.push(Block::new(format!("level {i}")));
            cursor = cursor.children.last_mut().unwrap();
        }
        let text = render(&blocks);
        assert_eq!(parse(&text).unwrap(), blocks);
    }

    /// Generatore pseudo-casuale deterministico (LCG) per un test
    /// property-based senza aggiungere `rand` come dipendenza.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
            self.0
        }
        fn range(&mut self, max: u64) -> u64 {
            self.next() % max.max(1)
        }
    }

    fn random_blocks(rng: &mut Lcg, depth: usize, count: usize) -> Vec<Block> {
        (0..count)
            .map(|i| {
                let words = rng.range(4) + 1;
                let content = (0..words)
                    .map(|w| format!("w{}{}{}", depth, i, w))
                    .collect::<Vec<_>>()
                    .join(" ");
                let children = if depth < 4 && rng.range(3) == 0 {
                    let child_count = (rng.range(3) + 1) as usize;
                    random_blocks(rng, depth + 1, child_count)
                } else {
                    Vec::new()
                };
                Block { content, children }
            })
            .collect()
    }

    #[test]
    fn property_random_trees_round_trip() {
        let mut rng = Lcg(0xC0FFEE);
        for seed in 0..200u64 {
            rng.0 ^= seed.wrapping_mul(0x9E3779B97F4A7C15);
            let root_count = (rng.range(5) + 1) as usize;
            let blocks = random_blocks(&mut rng, 0, root_count);
            let text = render(&blocks);
            assert_eq!(parse(&text).unwrap(), blocks, "failed for text:\n{text}");
        }
    }
}
