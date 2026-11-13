use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use regex::Regex;

use crate::model::{SymbolEntry, SymbolKind};
use super::SourceParser;

pub struct GoParser {
    fn_re: Regex,
    type_struct_re: Regex,
    type_alias_re: Regex,
    const_var_re: Regex,
}

impl GoParser {
    pub fn new() -> Self {
        Self {
            // func [ (r *Recv) ] ExportedFunc(params) [ (returns) | return ]
            fn_re: Regex::new(
                r"^\s*func\s+(?:\((?:[A-Za-z0-9_*\s,]+)\)\s+)?([A-Z][A-Za-z0-9_]*)\s*\((.*?)\)(?:\s*(?:([A-Za-z0-9_.*&\[\]]+)|\((.*?)\)))?"
            ).unwrap(),

            // type ExportedType struct|interface
            type_struct_re: Regex::new(
                r"^\s*type\s+([A-Z][A-Za-z0-9_]*)\s+(struct|interface)"
            ).unwrap(),

            // type ExportedType OtherType
            type_alias_re: Regex::new(
                r"^\s*type\s+([A-Z][A-Za-z0-9_]*)\s+([A-Za-z0-9_.*&\[\]]+)"
            ).unwrap(),

            // const|var ExportedName
            const_var_re: Regex::new(
                r"^\s*(?:const|var)\s+([A-Z][A-Za-z0-9_]*)"
            ).unwrap(),
        }
    }

    pub fn parse_content(&self, content: &str, relative_path: &str) -> Result<Vec<SymbolEntry>> {
        let mut symbols = Vec::new();
        let mut comment_buffer: Vec<String> = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Comentarios Go estilo //
            if trimmed.starts_with("//") {
                let clean = trimmed.trim_start_matches('/').trim();
                if !clean.is_empty() && !clean.starts_with("go:") && !clean.starts_with("+build") {
                    comment_buffer.push(clean.to_string());
                }
                continue;
            }

            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("package ") || trimmed.starts_with("import ") {
                comment_buffer.clear();
                continue;
            }

            let doc = if comment_buffer.is_empty() {
                None
            } else {
                Some(comment_buffer.join(" "))
            };

            // 1. Funciones y Métodos públicos (inician con Mayúscula)
            if let Some(caps) = self.fn_re.captures(trimmed) {
                let fn_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let params = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                let ret_single = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");
                let ret_multi = caps.get(4).map(|m| m.as_str().trim()).unwrap_or("");

                let returns = if !ret_single.is_empty() {
                    ret_single
                } else if !ret_multi.is_empty() {
                    ret_multi
                } else {
                    ""
                };

                let sig_tech = if returns.is_empty() {
                    format!("func {fn_name}({params})")
                } else {
                    format!("func {fn_name}({params}) -> ({returns})")
                };

                symbols.push(SymbolEntry {
                    name: format!("{fn_name}()"),
                    kind: SymbolKind::Function,
                    file_path: relative_path.to_string(),
                    signature_tech: sig_tech,
                    human_desc: None,
                    doc_comment: doc,
                });
                comment_buffer.clear();
                continue;
            }

            // 2. Structs o Interfaces públicas
            if let Some(caps) = self.type_struct_re.captures(trimmed) {
                let type_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let kind_str = caps.get(2).map(|m| m.as_str()).unwrap_or("struct");

                let kind = if kind_str == "interface" {
                    SymbolKind::Trait
                } else {
                    SymbolKind::Struct
                };

                symbols.push(SymbolEntry {
                    name: type_name.to_string(),
                    kind,
                    file_path: relative_path.to_string(),
                    signature_tech: format!("type {type_name} {kind_str}"),
                    human_desc: None,
                    doc_comment: doc,
                });
                comment_buffer.clear();
                continue;
            }

            // 3. Type Aliases / Definiciones
            if let Some(caps) = self.type_alias_re.captures(trimmed) {
                let type_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let target = caps.get(2).map(|m| m.as_str()).unwrap_or("");

                if target != "struct" && target != "interface" {
                    symbols.push(SymbolEntry {
                        name: type_name.to_string(),
                        kind: SymbolKind::TypeAlias,
                        file_path: relative_path.to_string(),
                        signature_tech: format!("type {type_name} = {target}"),
                        human_desc: None,
                        doc_comment: doc,
                    });
                    comment_buffer.clear();
                    continue;
                }
            }

            // 4. Const / Var públicas
            if let Some(caps) = self.const_var_re.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");

                symbols.push(SymbolEntry {
                    name: name.to_string(),
                    kind: SymbolKind::Const,
                    file_path: relative_path.to_string(),
                    signature_tech: format!("const/var {name}"),
                    human_desc: None,
                    doc_comment: doc,
                });
                comment_buffer.clear();
                continue;
            }

            comment_buffer.clear();
        }

        Ok(symbols)
    }
}

impl Default for GoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceParser for GoParser {
    fn can_parse(&self, path: &Path) -> bool {
        path.extension().and_then(|s| s.to_str()) == Some("go")
    }

    fn parse_file(&self, file_path: &Path, repo_root: &Path) -> Result<Vec<SymbolEntry>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("No se pudo leer archivo Go {}", file_path.display()))?;

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
    fn test_go_parser_extracts_exported_symbols() {
        let parser = GoParser::new();
        let code = r#"
package auth

import "context"

// TokenService gestiona tokens de acceso JWT.
type TokenService struct {
    secret []byte
}

// Authenticator define el contrato para validar credenciales.
type Authenticator interface {
    Authenticate(ctx context.Context) error
}

type UserID string

const DefaultTimeout = 30

// NewTokenService inicializa un nuevo servicio.
func NewTokenService(secret []byte) *TokenService {
    return &TokenService{secret: secret}
}

func (s *TokenService) ValidateToken(token string) (bool, error) {
    return true, nil
}

// función privada, debe ser ignorada
func internalHelper() {}
"#;

        let symbols = parser.parse_content(code, "pkg/auth/token.go").unwrap();

        assert_eq!(symbols.len(), 6);
        assert_eq!(symbols[0].name, "TokenService");
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
        assert_eq!(symbols[0].doc_comment.as_deref(), Some("TokenService gestiona tokens de acceso JWT."));

        assert_eq!(symbols[1].name, "Authenticator");
        assert_eq!(symbols[1].kind, SymbolKind::Trait);

        assert_eq!(symbols[2].name, "UserID");
        assert_eq!(symbols[2].kind, SymbolKind::TypeAlias);

        assert_eq!(symbols[3].name, "DefaultTimeout");
        assert_eq!(symbols[3].kind, SymbolKind::Const);

        assert_eq!(symbols[4].name, "NewTokenService()");
        assert_eq!(symbols[4].kind, SymbolKind::Function);
        assert_eq!(symbols[4].doc_comment.as_deref(), Some("NewTokenService inicializa un nuevo servicio."));

        assert_eq!(symbols[5].name, "ValidateToken()");
        assert_eq!(symbols[5].kind, SymbolKind::Function);
    }
}
