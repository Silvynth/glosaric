use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

use crate::format::markdown_parser::parse_markdown;
use crate::model::ModuleSection;

pub struct AuditReport {
    pub total_code_symbols: usize,
    pub total_doc_symbols: usize,
    pub missing_in_doc: Vec<String>,
    pub obsolete_in_doc: Vec<String>,
    pub pending_descriptions: Vec<String>,
}

impl AuditReport {
    pub fn is_clean(&self) -> bool {
        self.missing_in_doc.is_empty()
    }
}

pub fn audit_project(
    scanned_modules: &[ModuleSection],
    glosaric_path: &Path,
    project_name: &str,
) -> Result<AuditReport> {
    if !glosaric_path.exists() {
        anyhow::bail!(
            "No se encontró el archivo Glosaric en {}. Usa `glosaric init` primero.",
            glosaric_path.display()
        );
    }

    let content = fs::read_to_string(glosaric_path)
        .with_context(|| format!("No se pudo leer {}", glosaric_path.display()))?;

    let doc = parse_markdown(&content, project_name)?;

    let mut total_code = 0;
    let mut missing_in_doc = Vec::new();
    let mut pending_descriptions = Vec::new();

    // 1. Revisar cada símbolo del código contra el Glosaric
    for mod_sec in scanned_modules {
        for file_sec in &mod_sec.files {
            for sym in &file_sec.symbols {
                total_code += 1;
                let clean = sym.clean_name();
                match doc.find_symbol(&clean, &file_sec.file_path) {
                    Some(doc_sym) => {
                        // Verificar si tiene descripción pendiente
                        if let Some(ref human) = doc_sym.human_desc {
                            if human.contains("[Pendiente") {
                                pending_descriptions.push(format!(
                                    "{} :: {} ({})",
                                    file_sec.file_name, sym.name, human
                                ));
                            }
                        } else {
                            pending_descriptions.push(format!(
                                "{} :: {} (Sin descripción)",
                                file_sec.file_name, sym.name
                            ));
                        }
                    }
                    None => {
                        missing_in_doc.push(format!(
                            "{} :: {} [{}]",
                            file_sec.file_name, sym.name, sym.kind
                        ));
                    }
                }
            }
        }
    }

    // 2. Revisar símbolos en el Glosaric que ya no existen en el código
    let mut obsolete_in_doc = Vec::new();
    for mod_sec in &doc.modules {
        for file_sec in &mod_sec.files {
            for sym in &file_sec.symbols {
                let clean = sym.clean_name();
                let found_in_code = scanned_modules
                    .iter()
                    .flat_map(|m| &m.files)
                    .filter(|f| f.file_name == file_sec.file_name)
                    .flat_map(|f| &f.symbols)
                    .any(|s| s.clean_name() == clean);

                if !found_in_code {
                    obsolete_in_doc.push(format!("{} :: {}", file_sec.file_name, sym.name));
                }
            }
        }
    }

    Ok(AuditReport {
        total_code_symbols: total_code,
        total_doc_symbols: doc.total_symbols(),
        missing_in_doc,
        obsolete_in_doc,
        pending_descriptions,
    })
}
