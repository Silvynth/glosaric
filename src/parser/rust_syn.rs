use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use quote::ToTokens;
use syn::{
    Attribute, Expr, ExprLit, FnArg, ImplItem, Item, Lit, Meta, Pat, ReturnType, Signature,
    Visibility,
};

use crate::model::{SymbolEntry, SymbolKind};
use super::SourceParser;

pub struct RustSynParser;

impl RustSynParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustSynParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceParser for RustSynParser {
    fn can_parse(&self, path: &Path) -> bool {
        path.extension().and_then(|s| s.to_str()) == Some("rs")
    }

    fn parse_file(&self, file_path: &Path, repo_root: &Path) -> Result<Vec<SymbolEntry>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("No se pudo leer el archivo {}", file_path.display()))?;

        let syntax_tree = match syn::parse_file(&content) {
            Ok(st) => st,
            Err(err) => {
                // Si el archivo tiene errores de sintaxis o macros inusuales, logueamos y no rompemos
                eprintln!("⚠️  [glosaric] Advertencia parseando {}: {}", file_path.display(), err);
                return Ok(Vec::new());
            }
        };

        let relative_path = file_path
            .strip_prefix(repo_root)
            .unwrap_or(file_path)
            .to_string_lossy()
            .to_string();

        let mut symbols = Vec::new();

        for item in &syntax_tree.items {
            match item {
                Item::Fn(item_fn) => {
                    if is_public(&item_fn.vis) {
                        let (name, sig_tech) = format_fn_signature(&item_fn.sig);
                        let doc = extract_doc_comments(&item_fn.attrs);
                        symbols.push(SymbolEntry {
                            name,
                            kind: SymbolKind::Function,
                            file_path: relative_path.clone(),
                            signature_tech: sig_tech,
                            human_desc: None,
                            doc_comment: doc,
                        });
                    }
                }
                Item::Struct(item_struct) => {
                    if is_public(&item_struct.vis) {
                        let name = item_struct.ident.to_string();
                        let doc = extract_doc_comments(&item_struct.attrs);
                        let sig_tech = format!("Struct {}", name);
                        symbols.push(SymbolEntry {
                            name,
                            kind: SymbolKind::Struct,
                            file_path: relative_path.clone(),
                            signature_tech: sig_tech,
                            human_desc: None,
                            doc_comment: doc,
                        });
                    }
                }
                Item::Enum(item_enum) => {
                    if is_public(&item_enum.vis) {
                        let name = item_enum.ident.to_string();
                        let doc = extract_doc_comments(&item_enum.attrs);
                        let variants_count = item_enum.variants.len();
                        let sig_tech = format!("Enum de {} variantes", variants_count);
                        symbols.push(SymbolEntry {
                            name,
                            kind: SymbolKind::Enum,
                            file_path: relative_path.clone(),
                            signature_tech: sig_tech,
                            human_desc: None,
                            doc_comment: doc,
                        });
                    }
                }
                Item::Trait(item_trait) => {
                    if is_public(&item_trait.vis) {
                        let name = item_trait.ident.to_string();
                        let doc = extract_doc_comments(&item_trait.attrs);
                        let sig_tech = format!("Trait {}", name);
                        symbols.push(SymbolEntry {
                            name,
                            kind: SymbolKind::Trait,
                            file_path: relative_path.clone(),
                            signature_tech: sig_tech,
                            human_desc: None,
                            doc_comment: doc,
                        });
                    }
                }
                Item::Type(item_type) => {
                    if is_public(&item_type.vis) {
                        let name = item_type.ident.to_string();
                        let doc = extract_doc_comments(&item_type.attrs);
                        let ty_str = item_type.ty.to_token_stream().to_string();
                        let sig_tech = format!("Type alias = {}", ty_str);
                        symbols.push(SymbolEntry {
                            name,
                            kind: SymbolKind::TypeAlias,
                            file_path: relative_path.clone(),
                            signature_tech: sig_tech,
                            human_desc: None,
                            doc_comment: doc,
                        });
                    }
                }
                Item::Const(item_const) => {
                    if is_public(&item_const.vis) {
                        let name = item_const.ident.to_string();
                        let doc = extract_doc_comments(&item_const.attrs);
                        let ty_str = item_const.ty.to_token_stream().to_string();
                        let sig_tech = format!("Const: {}", ty_str);
                        symbols.push(SymbolEntry {
                            name,
                            kind: SymbolKind::Const,
                            file_path: relative_path.clone(),
                            signature_tech: sig_tech,
                            human_desc: None,
                            doc_comment: doc,
                        });
                    }
                }
                Item::Impl(item_impl) => {
                    // Procesar métodos públicos dentro de bloques impl
                    for impl_item in &item_impl.items {
                        if let ImplItem::Fn(impl_fn) = impl_item {
                            if is_public(&impl_fn.vis) {
                                let (name, sig_tech) = format_fn_signature(&impl_fn.sig);
                                let doc = extract_doc_comments(&impl_fn.attrs);
                                symbols.push(SymbolEntry {
                                    name,
                                    kind: SymbolKind::Function,
                                    file_path: relative_path.clone(),
                                    signature_tech: sig_tech,
                                    human_desc: None,
                                    doc_comment: doc,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(symbols)
    }
}

fn is_public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn extract_doc_comments(attrs: &[Attribute]) -> Option<String> {
    let mut docs = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(meta_nv) = &attr.meta {
                if let Expr::Lit(ExprLit { lit: Lit::Str(lit_str), .. }) = &meta_nv.value {
                    let val = lit_str.value();
                    let trimmed = val.trim();
                    if !trimmed.is_empty() {
                        docs.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    if docs.is_empty() {
        None
    } else {
        Some(docs.join(" "))
    }
}

fn format_fn_signature(sig: &Signature) -> (String, String) {
    let fn_name = sig.ident.to_string();

    let mut param_names = Vec::new();
    let mut param_types = Vec::new();

    for input in &sig.inputs {
        match input {
            FnArg::Receiver(r) => {
                if r.reference.is_some() {
                    if r.mutability.is_some() {
                        param_names.push("&mut self".to_string());
                        param_types.push("&mut self".to_string());
                    } else {
                        param_names.push("&self".to_string());
                        param_types.push("&self".to_string());
                    }
                } else {
                    param_names.push("self".to_string());
                    param_types.push("self".to_string());
                }
            }
            FnArg::Typed(pat_type) => {
                let name = match &*pat_type.pat {
                    Pat::Ident(pat_ident) => pat_ident.ident.to_string(),
                    other => other.to_token_stream().to_string(),
                };
                let ty = pat_type.ty.to_token_stream().to_string();
                param_names.push(name);
                param_types.push(ty);
            }
        }
    }

    // Nombre para el glosario: nombre(param1, param2)
    // Omitimos &self o self en la lista coloquial de parámetros si es un método
    let call_params: Vec<String> = param_names
        .into_iter()
        .filter(|p| p != "&self" && p != "&mut self" && p != "self")
        .collect();

    let name = format!("{}({})", fn_name, call_params.join(", "));

    // Firma técnica: fn(params) -> RetType
    let ret = match &sig.output {
        ReturnType::Default => "".to_string(),
        ReturnType::Type(_, ty) => format!(" -> {}", ty.to_token_stream()),
    };

    let sig_tech = format!("fn({}){}", param_types.join(", "), ret);

    (name, sig_tech)
}
