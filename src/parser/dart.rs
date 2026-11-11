use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use regex::Regex;

use crate::model::{SymbolEntry, SymbolKind};
use super::SourceParser;

pub struct DartParser {
    class_re: Regex,
    enum_re: Regex,
    method_re: Regex,
}

impl DartParser {
    pub fn new() -> Self {
        Self {
            // Captura: class NombreClass (extends/implements...)
            class_re: Regex::new(r"^\s*(?:abstract\s+)?class\s+([A-Za-z0-9]+)(?:\s+(?:extends|implements|with)\s+([A-Za-z0-9_\s,<>&?]+))?").unwrap(),
            // Captura: enum NombreEnum
            enum_re: Regex::new(r"^\s*enum\s+([A-Za-z0-9]+)").unwrap(),
            // Captura: [static] [Future<T>] nombreMetodo(params) [{]
            method_re: Regex::new(r"^\s*(?:(?:static|final|const|late)\s+)*(?:([A-Za-z0-9_<>,?]+)\s+)?([a-zA-Z][A-Za-z0-9]*)\s*\((.*?)\)\s*(?:async\*?|=>|\{)?").unwrap(),
        }
    }
}

impl Default for DartParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceParser for DartParser {
    fn can_parse(&self, path: &Path) -> bool {
        path.extension().and_then(|s| s.to_str()) == Some("dart")
    }

    fn parse_file(&self, file_path: &Path, repo_root: &Path) -> Result<Vec<SymbolEntry>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("No se pudo leer archivo Dart {}", file_path.display()))?;

        let relative_path = file_path
            .strip_prefix(repo_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        let mut symbols = Vec::new();
        let mut doc_accumulator: Vec<String> = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // 1. Acumular comentarios de documentación (/// o //)
            if trimmed.starts_with("///") || trimmed.starts_with("//") {
                let comment = trimmed
                    .trim_start_matches('/')
                    .trim()
                    .to_string();
                if !comment.is_empty() && !comment.starts_with("ignore:") {
                    doc_accumulator.push(comment);
                }
                continue;
            }

            // Si es línea vacía o import/export, limpiamos el acumulador si no hay definición inmediata
            if trimmed.is_empty() || trimmed.starts_with("import ") || trimmed.starts_with("export ") || trimmed.starts_with("part ") {
                if trimmed.is_empty() {
                    // Mantenemos hasta 1 línea vacía para comentarios de cabecera
                } else {
                    doc_accumulator.clear();
                }
                continue;
            }

            // 2. Detectar Clases Dart
            if let Some(caps) = self.class_re.captures(trimmed) {
                let class_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                // En Dart, lo que empieza con _ es privado
                if !class_name.starts_with('_') && !class_name.is_empty() {
                    let inheritance = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                    let sig_tech = if inheritance.is_empty() {
                        format!("Class {}", class_name)
                    } else {
                        format!("Class {} : {}", class_name, inheritance)
                    };

                    let doc = if doc_accumulator.is_empty() {
                        None
                    } else {
                        Some(doc_accumulator.join(" "))
                    };

                    symbols.push(SymbolEntry {
                        name: class_name.to_string(),
                        kind: SymbolKind::Struct, // Mapeamos class a Struct/Class
                        file_path: relative_path.clone(),
                        signature_tech: sig_tech,
                        human_desc: None,
                        doc_comment: doc,
                    });
                }
                doc_accumulator.clear();
                continue;
            }

            // 3. Detectar Enums Dart
            if let Some(caps) = self.enum_re.captures(trimmed) {
                let enum_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if !enum_name.starts_with('_') && !enum_name.is_empty() {
                    let doc = if doc_accumulator.is_empty() {
                        None
                    } else {
                        Some(doc_accumulator.join(" "))
                    };

                    symbols.push(SymbolEntry {
                        name: enum_name.to_string(),
                        kind: SymbolKind::Enum,
                        file_path: relative_path.clone(),
                        signature_tech: format!("Enum {}", enum_name),
                        human_desc: None,
                        doc_comment: doc,
                    });
                }
                doc_accumulator.clear();
                continue;
            }

            // 4. Detectar Métodos y Funciones reales de la clase o top-level
            // En Dart estándar, los métodos de clase están a nivel de indentación 2 o top-level (0).
            // Llamadas a widgets como SizedBox(16) o setState(() dentro de build() están indentadas a 4+ espacios.
            let indent_level = line.chars().take_while(|c| c.is_whitespace()).count();
            if indent_level > 2 {
                // Es código interno dentro de un método (ej. un setState o widget anidado)
                continue;
            }

            if let Some(caps) = self.method_re.captures(trimmed) {
                let method_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                // Ignorar palabras clave del lenguaje o widgets comunes en llamadas
                if is_dart_keyword(method_name) || is_common_flutter_widget_or_call(method_name) {
                    doc_accumulator.clear();
                    continue;
                }

                // En Dart: si empieza con _ es privado
                if !method_name.starts_with('_') && !method_name.is_empty() {
                    let ret_type = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("void");
                    let raw_params = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                    // Extraer nombres simples de parámetros
                    let clean_params = simplify_dart_params(raw_params);
                    let name = format!("{}({})", method_name, clean_params);
                    let sig_tech = format!("{}({}) -> {}", method_name, raw_params, ret_type);

                    let doc = if doc_accumulator.is_empty() {
                        None
                    } else {
                        Some(doc_accumulator.join(" "))
                    };

                    symbols.push(SymbolEntry {
                        name,
                        kind: SymbolKind::Function,
                        file_path: relative_path.clone(),
                        signature_tech: sig_tech,
                        human_desc: None,
                        doc_comment: doc,
                    });
                }
                doc_accumulator.clear();
                continue;
            }

            // Línea de código común: limpiamos comentarios huérfanos
            if !trimmed.starts_with('@') {
                doc_accumulator.clear();
            }
        }

        Ok(symbols)
    }
}

fn is_dart_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "else" | "switch" | "case" | "for" | "while" | "do" | "try" | "catch" | "return" | "throw" | "assert" | "new"
    )
}

fn is_common_flutter_widget_or_call(word: &str) -> bool {
    matches!(
        word,
        "setState" | "print" | "super" | "SnackBar" | "SizedBox" | "Text" | "Center" | "Padding" | "Container" | "Column" | "Row" | "Icon"
    )
}

fn simplify_dart_params(raw: &str) -> String {
    if raw.trim().is_empty() {
        return String::new();
    }

    let mut names = Vec::new();
    // Dividir por comas respetando genéricos simples
    for part in raw.split(',') {
        let trimmed = part.trim().trim_matches('{').trim_matches('}').trim_matches('[').trim_matches(']').trim();
        if trimmed.is_empty() {
            continue;
        }
        // Tomar el último token que suele ser el nombre de la variable (ej: "required String input" -> "input")
        if let Some(param_name) = trimmed.split_whitespace().last() {
            let clean = param_name.trim_start_matches("this.").split('=').next().unwrap_or(param_name).trim();
            if !clean.is_empty() {
                names.push(clean.to_string());
            }
        }
    }

    names.join(", ")
}
