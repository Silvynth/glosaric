use anyhow::Result;
use crate::model::{FileSection, GlosaricDoc, ModuleSection, SymbolEntry, SymbolKind};

pub fn parse_markdown(content: &str, default_project_name: &str) -> Result<GlosaricDoc> {
    let mut doc = GlosaricDoc::new(default_project_name);

    let mut current_module: Option<ModuleSection> = None;
    let mut current_file: Option<FileSection> = None;

    let lines: Vec<&str> = content.lines().collect();
    let mut in_frontmatter = false;
    let mut frontmatter_count = 0;

    for line in lines {
        let trimmed = line.trim();

        // Manejo de frontmatter
        if trimmed == "---" {
            frontmatter_count += 1;
            if frontmatter_count == 1 {
                in_frontmatter = true;
                continue;
            } else if frontmatter_count == 2 {
                in_frontmatter = false;
                continue;
            }
        }

        if in_frontmatter {
            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim().to_lowercase();
                let val = v.trim().trim_matches('"').trim_matches('\'').to_string();
                match key.as_str() {
                    "proyecto" => doc.project_name = val,
                    "tipo" => doc.doc_type = val,
                    "actualizado" => doc.updated_at = val,
                    _ => {}
                }
            }
            continue;
        }

        // Título principal
        if trimmed.starts_with("# ") {
            doc.title = trimmed.trim_start_matches('#').trim().to_string();
            continue;
        }

        // Módulos: ## 📂 `src/core/` (Descripción) o ## src/core/
        if trimmed.starts_with("## ") {
            // Guardar archivo previo si existe
            if let Some(file) = current_file.take() {
                if let Some(ref mut module) = current_module {
                    if !file.symbols.is_empty() {
                        module.files.push(file);
                    }
                }
            }
            // Guardar módulo previo si existe
            if let Some(module) = current_module.take() {
                if !module.files.is_empty() {
                    doc.modules.push(module);
                }
            }

            let header_text = trimmed.trim_start_matches('#').trim();
            let (mod_path, mod_desc) = parse_module_header(header_text);

            current_module = Some(ModuleSection {
                module_path: mod_path,
                description: mod_desc,
                files: Vec::new(),
            });
            continue;
        }

        // Archivos o Subsecciones: ### `state.rs` o ### Navegación
        if trimmed.starts_with("### ") {
            if let Some(file) = current_file.take() {
                if let Some(ref mut module) = current_module {
                    if !file.symbols.is_empty() {
                        module.files.push(file);
                    }
                }
            }

            let file_header = trimmed.trim_start_matches('#').trim();
            // Limpiar comillas y posibles anotaciones
            let first_token = file_header.split_whitespace().next().unwrap_or(file_header);
            let clean_name = first_token.trim_matches('`').trim().to_string();

            let mod_prefix = current_module
                .as_ref()
                .map(|m| m.module_path.as_str())
                .unwrap_or("src/");

            let file_path = if clean_name.contains('/') {
                clean_name.clone()
            } else if clean_name.ends_with(".rs") || clean_name.ends_with(".dart") || clean_name.ends_with(".ts") || clean_name.ends_with(".py") {
                format!("{}{}", mod_prefix, clean_name)
            } else {
                // Es un subtítulo conceptual/temático (ej: "Navegación e Ignición")
                clean_name.clone()
            };

            current_file = Some(FileSection {
                file_name: clean_name,
                file_path,
                symbols: Vec::new(),
            });
            continue;
        }

        // Entradas de símbolos: - `nombre` | tech | human
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            if let Some(entry) = parse_symbol_line(trimmed, current_file.as_ref().map(|f| f.file_path.as_str())) {
                if let Some(ref mut file) = current_file {
                    file.symbols.push(entry);
                }
            }
        }
    }

    // Cerrar archivo y módulo final
    if let Some(file) = current_file.take() {
        if let Some(ref mut module) = current_module {
            if !file.symbols.is_empty() {
                module.files.push(file);
            }
        }
    }
    if let Some(module) = current_module.take() {
        if !module.files.is_empty() {
            doc.modules.push(module);
        }
    }

    Ok(doc)
}

fn parse_module_header(raw: &str) -> (String, Option<String>) {
    // Quitar emoji de carpeta si está
    let clean = raw.trim_start_matches('📂').trim();

    // Puede ser `src/core/` (descripción) o src/core/ (descripción)
    let (path_part, desc_part) = if let Some(idx) = clean.find('(') {
        let (p, d) = clean.split_at(idx);
        (p.trim(), Some(d.trim().to_string()))
    } else {
        (clean, None)
    };

    let path = path_part.trim_matches('`').trim().to_string();
    let norm_path = if !path.ends_with('/') && !path.contains('.') {
        format!("{path}/")
    } else {
        path
    };

    (norm_path, desc_part)
}

fn parse_symbol_line(line: &str, current_file_path: Option<&str>) -> Option<SymbolEntry> {
    let content = line.trim_start_matches('-').trim_start_matches('*').trim();
    if content.is_empty() {
        return None;
    }

    let file_path = current_file_path.unwrap_or("unknown").to_string();

    // Formato canónico: Col1 | Col2 | Col3
    if content.contains('|') {
        let parts: Vec<&str> = content.split('|').map(|s| s.trim()).collect();
        match parts.len() {
            3.. => {
                let name = parts[0].to_string();
                let tech = parts[1].to_string();
                let human = parts[2..].join(" | ");
                let kind = guess_kind(&name, &tech);

                Some(SymbolEntry {
                    name,
                    kind,
                    file_path,
                    signature_tech: tech,
                    human_desc: Some(human),
                    doc_comment: None,
                })
            }
            2 => {
                // Formato de 2 columnas: Col1 | Col2
                let name = parts[0].to_string();
                let col2 = parts[1].to_string();
                let kind = guess_kind(&name, &col2);

                Some(SymbolEntry {
                    name,
                    kind,
                    file_path,
                    signature_tech: col2.clone(),
                    human_desc: Some(col2),
                    doc_comment: None,
                })
            }
            _ => None,
        }
    } else if let Some((name, desc)) = content.split_once(':') {
        // Formato antiguo: - `funcion()`: descripcion
        let name_str = name.trim().to_string();
        let desc_str = desc.trim().to_string();
        let kind = guess_kind(&name_str, &desc_str);

        Some(SymbolEntry {
            name: name_str,
            kind,
            file_path,
            signature_tech: desc_str.clone(),
            human_desc: Some(desc_str),
            doc_comment: None,
        })
    } else {
        None
    }
}

fn guess_kind(name: &str, tech: &str) -> SymbolKind {
    let lower_tech = tech.to_lowercase();
    let lower_name = name.to_lowercase();

    if lower_tech.contains("enum") || lower_name.contains("enum") {
        SymbolKind::Enum
    } else if lower_tech.contains("struct") || (!name.contains('(') && name.chars().next().map_or(false, |c| c.is_uppercase())) {
        SymbolKind::Struct
    } else if lower_tech.contains("trait") {
        SymbolKind::Trait
    } else if lower_tech.contains("const") {
        SymbolKind::Const
    } else if lower_tech.contains("type") {
        SymbolKind::TypeAlias
    } else if name.contains('(') || lower_tech.starts_with("fn") {
        SymbolKind::Function
    } else {
        SymbolKind::Other("symbol".to_string())
    }
}
