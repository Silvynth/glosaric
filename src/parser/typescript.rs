use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use regex::Regex;

use crate::model::{SymbolEntry, SymbolKind};
use super::SourceParser;

pub struct TypeScriptParser {
    fn_re: Regex,
    arrow_re: Regex,
    class_re: Regex,
    interface_re: Regex,
    type_re: Regex,
    enum_re: Regex,
}

impl TypeScriptParser {
    pub fn new() -> Self {
        Self {
            // export [default] [async] function nombre<T>(params): Retorno
            fn_re: Regex::new(
                r"^\s*export\s+(?:default\s+)?(?:async\s+)?function\s+([A-Za-z0-9_$]+)\s*(?:<[^>]+>)?\s*\((.*?)\)(?:\s*:\s*([^;{]+))?"
            ).unwrap(),

            // export const/let/var nombre [: Tipo] = [async] (params) =>
            arrow_re: Regex::new(
                r"^\s*export\s+(?:const|let|var)\s+([A-Za-z0-9_$]+)(?:\s*:\s*([^=]+))?\s*=\s*(?:async\s+)?(?:\((.*?)\)|[A-Za-z0-9_$]+)\s*=>"
            ).unwrap(),

            // export [default] [abstract] class Nombre [extends Base] [implements I...]
            class_re: Regex::new(
                r"^\s*export\s+(?:default\s+)?(?:abstract\s+)?class\s+([A-Za-z0-9_$]+)(?:\s+extends\s+([A-Za-z0-9_$.<>, ]+))?(?:\s+implements\s+([A-Za-z0-9_$.<>, ]+))?"
            ).unwrap(),

            // export [default] interface Nombre [extends Base]
            interface_re: Regex::new(
                r"^\s*export\s+(?:default\s+)?interface\s+([A-Za-z0-9_$]+)(?:\s+extends\s+([A-Za-z0-9_$.<>, ]+))?"
            ).unwrap(),

            // export type Nombre = ...
            type_re: Regex::new(
                r"^\s*export\s+type\s+([A-Za-z0-9_$]+)(?:\s*<[^>]+>)?\s*="
            ).unwrap(),

            // export [const] enum Nombre
            enum_re: Regex::new(
                r"^\s*export\s+(?:const\s+)?enum\s+([A-Za-z0-9_$]+)"
            ).unwrap(),
        }
    }

    pub fn parse_content(&self, content: &str, relative_path: &str) -> Result<Vec<SymbolEntry>> {
        let mut symbols = Vec::new();
        let mut jsdoc_buffer: Vec<String> = Vec::new();
        let mut in_jsdoc = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Manejo de bloques JSDoc /** ... */
            if trimmed.starts_with("/**") {
                jsdoc_buffer.clear();
                in_jsdoc = true;
                let rest = trimmed.trim_start_matches("/**").trim();
                if let Some(clean_end) = rest.strip_suffix("*/") {
                    let single = clean_end.trim().trim_start_matches('*').trim();
                    if !single.is_empty() {
                        jsdoc_buffer.push(single.to_string());
                    }
                    in_jsdoc = false;
                } else if !rest.is_empty() {
                    let clean = rest.trim_start_matches('*').trim();
                    if !clean.is_empty() {
                        jsdoc_buffer.push(clean.to_string());
                    }
                }
                continue;
            }

            if in_jsdoc {
                if let Some(clean_end) = trimmed.strip_suffix("*/") {
                    let clean = clean_end.trim().trim_start_matches('*').trim();
                    if !clean.is_empty() && !clean.starts_with('@') {
                        jsdoc_buffer.push(clean.to_string());
                    }
                    in_jsdoc = false;
                } else {
                    let clean = trimmed.trim_start_matches('*').trim();
                    if !clean.is_empty() && !clean.starts_with('@') {
                        jsdoc_buffer.push(clean.to_string());
                    }
                }
                continue;
            }

            // Comentarios de una sola línea //
            if trimmed.starts_with("//") {
                let comment = trimmed.trim_start_matches('/').trim();
                if !comment.is_empty() && !comment.starts_with('@') && !comment.starts_with("eslint") {
                    jsdoc_buffer.push(comment.to_string());
                }
                continue;
            }

            // Líneas vacías: no borramos el buffer de inmediato
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with("import ") {
                jsdoc_buffer.clear();
                continue;
            }

            let doc = if jsdoc_buffer.is_empty() {
                None
            } else {
                Some(jsdoc_buffer.join(" "))
            };

            // 1. Funciones regulares: export function foo(a, b): ret
            if let Some(caps) = self.fn_re.captures(trimmed) {
                let fn_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let params = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                let ret = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                let sig_tech = if ret.is_empty() {
                    format!("fn {fn_name}({params})")
                } else {
                    format!("fn {fn_name}({params}) -> {ret}")
                };

                symbols.push(SymbolEntry {
                    name: format!("{fn_name}()"),
                    kind: SymbolKind::Function,
                    file_path: relative_path.to_string(),
                    signature_tech: sig_tech,
                    human_desc: None,
                    doc_comment: doc,
                });
                jsdoc_buffer.clear();
                continue;
            }

            // 2. Funciones flecha: export const foo = (a, b) =>
            if let Some(caps) = self.arrow_re.captures(trimmed) {
                let fn_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let type_annot = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                let params = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                let sig_tech = if !type_annot.is_empty() {
                    format!("const {fn_name}: {type_annot}")
                } else {
                    format!("const {fn_name}({params}) => ...")
                };

                symbols.push(SymbolEntry {
                    name: format!("{fn_name}()"),
                    kind: SymbolKind::Function,
                    file_path: relative_path.to_string(),
                    signature_tech: sig_tech,
                    human_desc: None,
                    doc_comment: doc,
                });
                jsdoc_buffer.clear();
                continue;
            }

            // 3. Clases: export class Foo [extends B] [implements I]
            if let Some(caps) = self.class_re.captures(trimmed) {
                let class_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let ext = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                let imp = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                let mut sig_tech = format!("class {class_name}");
                if !ext.is_empty() {
                    sig_tech.push_str(&format!(" extends {ext}"));
                }
                if !imp.is_empty() {
                    sig_tech.push_str(&format!(" implements {imp}"));
                }

                symbols.push(SymbolEntry {
                    name: class_name.to_string(),
                    kind: SymbolKind::Struct,
                    file_path: relative_path.to_string(),
                    signature_tech: sig_tech,
                    human_desc: None,
                    doc_comment: doc,
                });
                jsdoc_buffer.clear();
                continue;
            }

            // 4. Interfaces: export interface Foo [extends B]
            if let Some(caps) = self.interface_re.captures(trimmed) {
                let iface_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let ext = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");

                let sig_tech = if ext.is_empty() {
                    format!("interface {iface_name}")
                } else {
                    format!("interface {iface_name} extends {ext}")
                };

                symbols.push(SymbolEntry {
                    name: iface_name.to_string(),
                    kind: SymbolKind::Trait,
                    file_path: relative_path.to_string(),
                    signature_tech: sig_tech,
                    human_desc: None,
                    doc_comment: doc,
                });
                jsdoc_buffer.clear();
                continue;
            }

            // 5. Types: export type Foo = ...
            if let Some(caps) = self.type_re.captures(trimmed) {
                let type_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");

                symbols.push(SymbolEntry {
                    name: type_name.to_string(),
                    kind: SymbolKind::TypeAlias,
                    file_path: relative_path.to_string(),
                    signature_tech: format!("type {type_name}"),
                    human_desc: None,
                    doc_comment: doc,
                });
                jsdoc_buffer.clear();
                continue;
            }

            // 6. Enums: export enum Foo
            if let Some(caps) = self.enum_re.captures(trimmed) {
                let enum_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");

                symbols.push(SymbolEntry {
                    name: enum_name.to_string(),
                    kind: SymbolKind::Enum,
                    file_path: relative_path.to_string(),
                    signature_tech: format!("enum {enum_name}"),
                    human_desc: None,
                    doc_comment: doc,
                });
                jsdoc_buffer.clear();
                continue;
            }

            // Si es otra línea de código no exportada, limpiamos el doc acumulado
            if !trimmed.starts_with("export ") {
                jsdoc_buffer.clear();
            }
        }

        Ok(symbols)
    }
}

impl Default for TypeScriptParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceParser for TypeScriptParser {
    fn can_parse(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs")
        )
    }

    fn parse_file(&self, file_path: &Path, repo_root: &Path) -> Result<Vec<SymbolEntry>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("No se pudo leer archivo TypeScript/JavaScript {}", file_path.display()))?;

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
    fn test_ts_parser_extracts_exported_symbols() {
        let parser = TypeScriptParser::new();
        let code = r#"
import { useState } from 'react';

/**
 * Servicio de autenticación principal
 */
export class AuthService implements IAuth {
    login() {}
}

export interface UserSession {
    token: string;
    expiresAt: number;
}

export type Role = 'admin' | 'user';

export enum Status {
    Active,
    Inactive,
}

/**
 * Valida un token JWT
 * @param token Token a verificar
 */
export async function verifyToken(token: string): Promise<boolean> {
    return true;
}

export const logout = (userId: string) => {
    console.log(userId);
};

// Función privada interna (debe ser ignorada)
function helperInternal() {}
"#;

        let symbols = parser.parse_content(code, "src/auth.ts").unwrap();

        assert_eq!(symbols.len(), 6);
        assert_eq!(symbols[0].name, "AuthService");
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
        assert_eq!(symbols[0].doc_comment.as_deref(), Some("Servicio de autenticación principal"));

        assert_eq!(symbols[1].name, "UserSession");
        assert_eq!(symbols[1].kind, SymbolKind::Trait);

        assert_eq!(symbols[2].name, "Role");
        assert_eq!(symbols[2].kind, SymbolKind::TypeAlias);

        assert_eq!(symbols[3].name, "Status");
        assert_eq!(symbols[3].kind, SymbolKind::Enum);

        assert_eq!(symbols[4].name, "verifyToken()");
        assert_eq!(symbols[4].kind, SymbolKind::Function);
        assert_eq!(symbols[4].doc_comment.as_deref(), Some("Valida un token JWT"));

        assert_eq!(symbols[5].name, "logout()");
        assert_eq!(symbols[5].kind, SymbolKind::Function);
    }
}
