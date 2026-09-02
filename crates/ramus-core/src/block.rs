use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Testo markdown inline del blocco, senza il prefisso "- ".
    pub content: String,
    pub children: Vec<Block>,
}

impl Block {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    /// Path relativo alla radice del vault (es. "journals/2026-09-01.md").
    pub path: PathBuf,
    pub blocks: Vec<Block>,
}
