pub mod dart;
pub mod go;
pub mod python;
pub mod rust_syn;
pub mod typescript;

use std::path::Path;
use anyhow::Result;
use crate::model::SymbolEntry;

pub trait SourceParser: Send + Sync {
    /// Indica si este parser soporta el archivo dado por su extensión o contenido
    fn can_parse(&self, path: &Path) -> bool;

    /// Parsea el archivo y devuelve los símbolos públicos encontrados
    fn parse_file(&self, file_path: &Path, repo_root: &Path) -> Result<Vec<SymbolEntry>>;
}

/// Obtiene todos los parsers disponibles en el sistema
pub fn default_parsers() -> Vec<Box<dyn SourceParser>> {
    vec![
        Box::new(rust_syn::RustSynParser::new()),
        Box::new(dart::DartParser::new()),
        Box::new(python::PythonParser::new()),
        Box::new(typescript::TypeScriptParser::new()),
        Box::new(go::GoParser::new()),
    ]
}
