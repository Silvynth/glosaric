use std::collections::BTreeMap;
use std::path::Path;
use anyhow::Result;
use walkdir::WalkDir;

use crate::model::{FileSection, ModuleSection, SymbolEntry};
use crate::parser::{default_parsers, SourceParser};

pub struct CodeScanner {
    parsers: Vec<Box<dyn SourceParser>>,
}

impl CodeScanner {
    pub fn new() -> Self {
        Self {
            parsers: default_parsers(),
        }
    }

    /// Escanea recursivamente el proyecto y organiza los símbolos por módulo y archivo
    pub fn scan_project(&self, repo_root: &Path) -> Result<Vec<ModuleSection>> {
        let mut module_map: BTreeMap<String, BTreeMap<String, Vec<SymbolEntry>>> = BTreeMap::new();

        let walker = WalkDir::new(repo_root)
            .into_iter()
            .filter_entry(|e| !is_ignored_dir(e.path()));

        for entry_res in walker {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };

            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Buscar un parser adecuado
            for parser in &self.parsers {
                if parser.can_parse(path) {
                    if let Ok(symbols) = parser.parse_file(path, repo_root) {
                        if !symbols.is_empty() {
                            let rel_path = path.strip_prefix(repo_root).unwrap_or(path);
                            let file_name = path
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();

                            let parent_dir = rel_path
                                .parent()
                                .and_then(|p| p.to_str())
                                .map(|s| {
                                    if s.is_empty() {
                                        "src/".to_string()
                                    } else if !s.ends_with('/') {
                                        format!("{s}/")
                                    } else {
                                        s.to_string()
                                    }
                                })
                                .unwrap_or_else(|| "src/".to_string());

                            module_map
                                .entry(parent_dir)
                                .or_default()
                                .entry(file_name)
                                .or_default()
                                .extend(symbols);
                        }
                    }
                    break;
                }
            }
        }

        let mut modules = Vec::new();
        for (mod_path, files_map) in module_map {
            let mut files = Vec::new();
            for (file_name, symbols) in files_map {
                let file_path = format!("{}{}", mod_path, file_name);
                files.push(FileSection {
                    file_name,
                    file_path,
                    symbols,
                });
            }
            modules.push(ModuleSection {
                module_path: mod_path,
                description: None,
                files,
            });
        }

        Ok(modules)
    }
}

impl Default for CodeScanner {
    fn default() -> Self {
        Self::new()
    }
}

fn is_ignored_dir(path: &Path) -> bool {
    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
        matches!(
            file_name,
            "target"
                | ".git"
                | "node_modules"
                | "build"
                | ".dart_tool"
                | "dist"
                | ".cache"
                | ".idea"
                | ".vscode"
        )
    } else {
        false
    }
}
