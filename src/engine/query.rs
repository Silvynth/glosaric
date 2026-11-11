use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::format::markdown_parser::parse_markdown;
use crate::model::SymbolEntry;

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResult {
    pub query: String,
    pub matches_count: usize,
    pub results: Vec<SymbolEntry>,
}

pub fn search_glosaric(
    glosaric_path: &Path,
    term: &str,
    project_name: &str,
) -> Result<QueryResult> {
    if !glosaric_path.exists() {
        anyhow::bail!(
            "No se encontró el archivo Glosaric en {}. Usa `glosaric init` primero.",
            glosaric_path.display()
        );
    }

    let content = fs::read_to_string(glosaric_path)
        .with_context(|| format!("No se pudo leer {}", glosaric_path.display()))?;

    let doc = parse_markdown(&content, project_name)?;
    let term_lower = term.to_lowercase();

    let mut matches = Vec::new();

    for module in &doc.modules {
        for file in &module.files {
            for sym in &file.symbols {
                let name_match = sym.name.to_lowercase().contains(&term_lower);
                let tech_match = sym.signature_tech.to_lowercase().contains(&term_lower);
                let human_match = sym
                    .human_desc
                    .as_ref()
                    .map_or(false, |h| h.to_lowercase().contains(&term_lower));

                if name_match || tech_match || human_match {
                    matches.push(sym.clone());
                }
            }
        }
    }

    Ok(QueryResult {
        query: term.to_string(),
        matches_count: matches.len(),
        results: matches,
    })
}
