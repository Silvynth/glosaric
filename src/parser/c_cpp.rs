use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use regex::Regex;

use crate::model::{SymbolEntry, SymbolKind};
use super::SourceParser;

pub struct CCppParser {
    fn_re: Regex,
    struct_class_re: Regex,
    enum_re: Regex,
    define_re: Regex,
}

impl CCppParser {
    pub fn new() -> Self {
        Self {
            // Captura funciones típicas C/C++: [tipo] nombre(params) [{ o ;]
            // Ignora palabras clave reservadas de control
            fn_re: Regex::new(
                r"^\s*(?:(?:inline|virtual|static|extern|explicit|constexpr|friend)\s+)*(?:[A-Za-z0-9_&*:<>,]+(?:\s+[*&]+)?\s+)+([a-zA-Z_][a-zA-Z0-9_]*)\s*\((.*?)\)\s*(?:const|noexcept|override|final)*\s*(?:\{|;|=|\n|$)"
            ).unwrap(),

            // Captura: [typedef] struct|class|union Nombre
            struct_class_re: Regex::new(
                r"^\s*(?:typedef\s+)?(struct|class|union)\s+([A-Za-z0-9_]+)(?:\s*:\s*(?:public|protected|private)\s+([A-Za-z0-9_:<>, ]+))?"
            ).unwrap(),

            // Captura: [typedef] enum [class] Nombre
            enum_re: Regex::new(
                r"^\s*(?:typedef\s+)?enum\s+(?:class\s+|struct\s+)?([A-Za-z0-9_]+)"
            ).unwrap(),

            // Captura macros #define NOMBRE(...)
            define_re: Regex::new(
                r"^\s*#\s*define\s+([A-Z0-9_]+)\s*(?:\((.*?)\))?"
            ).unwrap(),
        }
    }

    pub fn parse_content(&self, content: &str, relative_path: &str) -> Result<Vec<SymbolEntry>> {
        let mut symbols = Vec::new();
        let mut comment_buffer: Vec<String> = Vec::new();
        let mut in_multiline_comment = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Bloques de comentario /* ... */
            if trimmed.starts_with("/*") {
                comment_buffer.clear();
                in_multiline_comment = true;
                let rest = trimmed.trim_start_matches("/*").trim();
                if let Some(clean_end) = rest.strip_suffix("*/") {
                    let single = clean_end.trim().trim_start_matches('*').trim();
                    if !single.is_empty() {
                        comment_buffer.push(single.to_string());
                    }
                    in_multiline_comment = false;
                } else if !rest.is_empty() {
                    let clean = rest.trim_start_matches('*').trim();
                    if !clean.is_empty() {
                        comment_buffer.push(clean.to_string());
                    }
                }
                continue;
            }

            if in_multiline_comment {
                if let Some(clean_end) = trimmed.strip_suffix("*/") {
                    let clean = clean_end.trim().trim_start_matches('*').trim();
                    if !clean.is_empty() && !clean.starts_with('@') {
                        comment_buffer.push(clean.to_string());
                    }
                    in_multiline_comment = false;
                } else {
                    let clean = trimmed.trim_start_matches('*').trim();
                    if !clean.is_empty() && !clean.starts_with('@') {
                        comment_buffer.push(clean.to_string());
                    }
                }
                continue;
            }

            // Comentarios de una sola línea //
            if trimmed.starts_with("//") {
                let clean = trimmed.trim_start_matches('/').trim();
                if !clean.is_empty() && !clean.starts_with('@') {
                    comment_buffer.push(clean.to_string());
                }
                continue;
            }

            if trimmed.is_empty() || trimmed.starts_with("#include") {
                if trimmed.starts_with("#include") {
                    comment_buffer.clear();
                }
                continue;
            }

            let doc = if comment_buffer.is_empty() {
                None
            } else {
                Some(comment_buffer.join(" "))
            };

            // 1. Structs, Classes, Unions
            if let Some(caps) = self.struct_class_re.captures(trimmed) {
                let kind_str = caps.get(1).map(|m| m.as_str()).unwrap_or("struct");
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let inheritance = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                // Ignorar forward declarations de 1 línea tipo "struct Foo;"
                let is_forward_decl = trimmed.ends_with(';') && !trimmed.contains('{');
                if !name.is_empty() && !is_forward_decl {
                    let sig_tech = if inheritance.is_empty() {
                        format!("{kind_str} {name}")
                    } else {
                        format!("{kind_str} {name} : {inheritance}")
                    };

                    symbols.push(SymbolEntry {
                        name: name.to_string(),
                        kind: SymbolKind::Struct,
                        file_path: relative_path.to_string(),
                        signature_tech: sig_tech,
                        human_desc: None,
                        doc_comment: doc,
                    });
                    comment_buffer.clear();
                    continue;
                }
            }

            // 2. Enums
            if let Some(caps) = self.enum_re.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let is_forward_decl = trimmed.ends_with(';') && !trimmed.contains('{');
                if !name.is_empty() && !is_forward_decl {
                    symbols.push(SymbolEntry {
                        name: name.to_string(),
                        kind: SymbolKind::Enum,
                        file_path: relative_path.to_string(),
                        signature_tech: format!("enum {name}"),
                        human_desc: None,
                        doc_comment: doc,
                    });
                    comment_buffer.clear();
                    continue;
                }
            }

            // 3. Macros #define
            if let Some(caps) = self.define_re.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let params = caps.get(2).map(|m| m.as_str().trim());

                // Ignorar include guards tipo #define FOO_H
                let is_header_guard = name.ends_with("_H") || name.ends_with("_HPP");
                if !name.is_empty() && !is_header_guard {
                    let sig_tech = match params {
                        Some(p) if !p.is_empty() => format!("#define {name}({p})"),
                        _ => format!("#define {name}"),
                    };

                    symbols.push(SymbolEntry {
                        name: name.to_string(),
                        kind: SymbolKind::Macro,
                        file_path: relative_path.to_string(),
                        signature_tech: sig_tech,
                        human_desc: None,
                        doc_comment: doc,
                    });
                    comment_buffer.clear();
                    continue;
                }
            }

            // 4. Funciones
            if let Some(caps) = self.fn_re.captures(trimmed) {
                let fn_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let params = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");

                // Filtrar palabras clave comunes que regex puede confundir
                if !is_c_control_keyword(fn_name) && !fn_name.is_empty() {
                    // Ignorar funciones estáticas privadas (static void helper())
                    if !trimmed.starts_with("static ") {
                        symbols.push(SymbolEntry {
                            name: format!("{fn_name}()"),
                            kind: SymbolKind::Function,
                            file_path: relative_path.to_string(),
                            signature_tech: format!("fn {fn_name}({params})"),
                            human_desc: None,
                            doc_comment: doc,
                        });
                        comment_buffer.clear();
                        continue;
                    }
                }
            }

            comment_buffer.clear();
        }

        Ok(symbols)
    }
}

fn is_c_control_keyword(word: &str) -> bool {
    matches!(
        word,
        "if" | "else" | "switch" | "case" | "for" | "while" | "do" | "return" | "sizeof" | "typedef"
    )
}

impl Default for CCppParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceParser for CCppParser {
    fn can_parse(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("c" | "h" | "cpp" | "hpp" | "cc" | "cxx" | "hh")
        )
    }

    fn parse_file(&self, file_path: &Path, repo_root: &Path) -> Result<Vec<SymbolEntry>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("No se pudo leer archivo C/C++ {}", file_path.display()))?;

        let relative_path = file_path
            .strip_prefix(repo_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        self.parse_content(&content, &relative_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_cpp_parser_extracts_symbols() {
        let parser = CCppParser::new();
        let code = r#"
#ifndef COSMOS_HUD_H
#define COSMOS_HUD_H

#include <stdio.h>

#define MAX_PARTICLES 1024
#define CLAMP(x, min, max) ((x) < (min) ? (min) : ((x) > (max) ? (max) : (x)))

/**
 * Representa el estado de una estrella volátil en el HUD.
 */
typedef struct CosmosStar {
    float x;
    float y;
    float brightness;
} CosmosStar;

enum RenderMode {
    RENDER_ISOMETRIC,
    RENDER_PERSPECTIVE
};

class GalaxyRenderer : public IRenderer {
public:
    void render();
};

/*
 * Inicializa el subsistema de Wayland y EGL.
 */
int cosmos_init_display(const char* socket_name, int width, int height);

void cosmos_destroy_context(void);

static void private_helper(void);
#endif
"#;

        let symbols = parser.parse_content(code, "include/cosmos.h").unwrap();

        assert_eq!(symbols.len(), 8);
        assert_eq!(symbols[0].name, "MAX_PARTICLES");
        assert_eq!(symbols[0].kind, SymbolKind::Macro);

        assert_eq!(symbols[1].name, "CLAMP");
        assert_eq!(symbols[1].kind, SymbolKind::Macro);

        assert_eq!(symbols[2].name, "CosmosStar");
        assert_eq!(symbols[2].kind, SymbolKind::Struct);
        assert_eq!(symbols[2].doc_comment.as_deref(), Some("Representa el estado de una estrella volátil en el HUD."));

        assert_eq!(symbols[3].name, "RenderMode");
        assert_eq!(symbols[3].kind, SymbolKind::Enum);

        assert_eq!(symbols[4].name, "GalaxyRenderer");
        assert_eq!(symbols[4].kind, SymbolKind::Struct);

        assert_eq!(symbols[5].name, "render()");
        assert_eq!(symbols[5].kind, SymbolKind::Function);

        assert_eq!(symbols[6].name, "cosmos_init_display()");
        assert_eq!(symbols[6].kind, SymbolKind::Function);
        assert_eq!(symbols[6].doc_comment.as_deref(), Some("Inicializa el subsistema de Wayland y EGL."));

        assert_eq!(symbols[7].name, "cosmos_destroy_context()");
        assert_eq!(symbols[7].kind, SymbolKind::Function);
    }
}
