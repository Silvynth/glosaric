use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use regex::Regex;

use crate::model::{SymbolEntry, SymbolKind};
use super::SourceParser;

pub struct PythonParser {
    class_re: Regex,
    def_re: Regex,
}

impl PythonParser {
    pub fn new() -> Self {
        Self {
            class_re: Regex::new(r"^\s*class\s+([A-Za-z0-9_]+)(?:\((.*?)\))?\s*:").unwrap(),
            def_re: Regex::new(r"^\s*(?:async\s+)?def\s+([A-Za-z0-9_]+)\s*\((.*?)\)(?:\s*->\s*([^:]+))?\s*:").unwrap(),
        }
    }
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceParser for PythonParser {
    fn can_parse(&self, path: &Path) -> bool {
        path.extension().and_then(|s| s.to_str()) == Some("py")
    }

    fn parse_file(&self, file_path: &Path, repo_root: &Path) -> Result<Vec<SymbolEntry>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("No se pudo leer archivo Python {}", file_path.display()))?;

        let relative_path = file_path
            .strip_prefix(repo_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        let mut symbols = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut comment_acc: Vec<String> = Vec::new();

        let mut i = 0;
        while i < lines.len() {
            let raw_line = lines[i];
            let trimmed = raw_line.trim();

            // Comentarios precedentes estilo #
            if trimmed.starts_with('#') {
                let clean = trimmed.trim_start_matches('#').trim().to_string();
                if !clean.is_empty() && !clean.starts_with("coding:") && !clean.starts_with("type:") {
                    comment_acc.push(clean);
                }
                i += 1;
                continue;
            }

            if trimmed.is_empty() {
                // mantenemos si hay comentario acumulado reciente
                i += 1;
                continue;
            }

            let indent_level = raw_line.chars().take_while(|c| c.is_whitespace()).count();

            // 1. Detectar Clases Python (generalmente a nivel 0 de indentación)
            if indent_level <= 4 {
                if let Some(caps) = self.class_re.captures(trimmed) {
                    let class_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    // Ignorar clases privadas que empiezan con _ (a menos que no sea solo _ )
                    if !class_name.starts_with('_') && !class_name.is_empty() {
                        let bases = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                        let sig_tech = if bases.is_empty() {
                            format!("Class {}", class_name)
                        } else {
                            format!("Class {}({})", class_name, bases)
                        };

                        // Buscar docstring inmediatamente debajo
                        let (doc, next_i) = extract_python_docstring(&lines, i + 1, &comment_acc);
                        i = next_i;
                        comment_acc.clear();

                        symbols.push(SymbolEntry {
                            name: class_name.to_string(),
                            kind: SymbolKind::Struct,
                            file_path: relative_path.clone(),
                            signature_tech: sig_tech,
                            human_desc: None,
                            doc_comment: doc,
                        });
                        continue;
                    }
                }
            }

            // 2. Detectar Funciones / Métodos (nivel 0 para top-level, nivel 4 para métodos de clase)
            if indent_level <= 4 {
                if let Some(caps) = self.def_re.captures(trimmed) {
                    let fn_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    // Mostrar __init__ o métodos públicos (que no empiezan con _)
                    let is_public = !fn_name.starts_with('_') || fn_name == "__init__";

                    if is_public && !fn_name.is_empty() {
                        let raw_params = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                        let ret_type = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                        let clean_params = simplify_python_params(raw_params);
                        let name = format!("{}({})", fn_name, clean_params);

                        let ret_str = if ret_type.is_empty() {
                            String::new()
                        } else {
                            format!(" -> {}", ret_type)
                        };
                        let sig_tech = format!("def {}({}){}", fn_name, raw_params, ret_str);

                        let (doc, next_i) = extract_python_docstring(&lines, i + 1, &comment_acc);
                        i = next_i;
                        comment_acc.clear();

                        symbols.push(SymbolEntry {
                            name,
                            kind: SymbolKind::Function,
                            file_path: relative_path.clone(),
                            signature_tech: sig_tech,
                            human_desc: None,
                            doc_comment: doc,
                        });
                        continue;
                    }
                }
            }

            comment_acc.clear();
            i += 1;
        }

        Ok(symbols)
    }
}

fn extract_python_docstring(
    lines: &[&str],
    mut idx: usize,
    preceding_comments: &[String],
) -> (Option<String>, usize) {
    // Si hay comentarios previos con #, los usamos como fallback
    let fallback = if preceding_comments.is_empty() {
        None
    } else {
        Some(preceding_comments.join(" "))
    };

    if idx >= lines.len() {
        return (fallback, idx);
    }

    let line = lines[idx].trim();
    if line.starts_with("\"\"\"") || line.starts_with("'''") {
        let quote = if line.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };
        let after_start = line.trim_start_matches(quote);

        // Docstring de una sola línea: """Texto aquí"""
        if after_start.contains(quote) {
            let doc = after_start.split(quote).next().unwrap_or("").trim().to_string();
            return (Some(doc), idx + 1);
        }

        // Docstring multilínea
        let mut doc_lines = Vec::new();
        if !after_start.trim().is_empty() {
            doc_lines.push(after_start.trim().to_string());
        }

        idx += 1;
        while idx < lines.len() {
            let l = lines[idx].trim();
            if l.contains(quote) {
                let before_end = l.split(quote).next().unwrap_or("").trim();
                if !before_end.is_empty() {
                    doc_lines.push(before_end.to_string());
                }
                idx += 1;
                break;
            } else {
                if !l.is_empty() {
                    doc_lines.push(l.to_string());
                }
                idx += 1;
            }
        }

        let doc = doc_lines.join(" ");
        return (Some(doc), idx);
    }

    (fallback, idx)
}

fn simplify_python_params(raw: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }

    let mut names = Vec::new();
    for part in raw.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() || trimmed == "self" || trimmed == "cls" {
            continue;
        }
        let param_name = trimmed.split(':').next().unwrap_or(trimmed);
        let clean = param_name.split('=').next().unwrap_or(param_name).trim();
        if !clean.is_empty() {
            names.push(clean.to_string());
        }
    }

    names.join(", ")
}
