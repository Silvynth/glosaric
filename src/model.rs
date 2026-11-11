use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Const,
    Macro,
    Other(String),
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "fn"),
            SymbolKind::Struct => write!(f, "struct"),
            SymbolKind::Enum => write!(f, "enum"),
            SymbolKind::Trait => write!(f, "trait"),
            SymbolKind::TypeAlias => write!(f, "type"),
            SymbolKind::Const => write!(f, "const"),
            SymbolKind::Macro => write!(f, "macro"),
            SymbolKind::Other(o) => write!(f, "{o}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolEntry {
    /// Nombre del símbolo con o sin llamada (ej: `open_hestia()`, `CockpitDomain`)
    pub name: String,
    /// Categoría del símbolo (fn, struct, enum, etc.)
    pub kind: SymbolKind,
    /// Ruta del archivo relativa al repositorio (ej: `src/core/state.rs`)
    pub file_path: String,
    /// Explicación técnica precisa o firma de tipos (Columna 2)
    pub signature_tech: String,
    /// Traducción humana / para qué sirve en la práctica (Columna 3)
    pub human_desc: Option<String>,
    /// Docstrings originales extraídos del código (/// o /** */)
    pub doc_comment: Option<String>,
}

impl SymbolEntry {
    /// Formatea la línea al estándar canónico:
    /// `- `simbolo(params)` | Explicación técnica precisa | Traducción humana`
    pub fn to_markdown_line(&self) -> String {
        let name_fmt = if self.name.starts_with('`') && self.name.ends_with('`') {
            self.name.clone()
        } else {
            format!("`{}`", self.name)
        };

        let tech = if self.signature_tech.trim().is_empty() {
            format!("{} {}", self.kind, self.clean_name())
        } else {
            self.signature_tech.trim().to_string()
        };

        let human = match &self.human_desc {
            Some(desc) if !desc.trim().is_empty() => desc.trim().to_string(),
            _ => match &self.doc_comment {
                Some(comment) if !comment.trim().is_empty() => comment.trim().to_string(),
                _ => "[Pendiente de documentar]".to_string(),
            },
        };

        format!("- {} | {} | {}", name_fmt, tech, human)
    }

    /// Nombre limpio sin comillas ni paréntesis para comparaciones
    pub fn clean_name(&self) -> String {
        self.name
            .trim_matches('`')
            .split('(')
            .next()
            .unwrap_or(&self.name)
            .trim()
            .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSection {
    pub file_name: String,
    pub file_path: String,
    pub symbols: Vec<SymbolEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSection {
    /// Ruta de módulo/directorio (ej: `src/core/`)
    pub module_path: String,
    /// Descripción diegética u opcional (ej: `(Núcleo, Estado, Ciclo CRT e Ignición)`)
    pub description: Option<String>,
    pub files: Vec<FileSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlosaricDoc {
    pub project_name: String,
    pub doc_type: String,
    pub updated_at: String,
    pub title: String,
    pub mission: String,
    pub modules: Vec<ModuleSection>,
}

impl GlosaricDoc {
    pub fn new(project_name: &str) -> Self {
        let today = chrono_lite_today();
        Self {
            project_name: project_name.to_string(),
            doc_type: "Glosario de Navegación Rápida".to_string(),
            updated_at: today.clone(),
            title: format!("📖 Glosaric // {} — Mapa Rápido de Símbolos y Contratos", project_name),
            mission: "Mapa de cabina y contrato vivo para localización instantánea de símbolos, structs y callbacks (< 15s con `Ctrl + F`), evitando quemar tokens en lecturas redundantes del árbol de código.".to_string(),
            modules: Vec::new(),
        }
    }

    /// Cuenta total de símbolos registrados en todo el documento
    pub fn total_symbols(&self) -> usize {
        self.modules
            .iter()
            .flat_map(|m| m.files.iter())
            .map(|f| f.symbols.len())
            .sum()
    }

    /// Busca un símbolo existente por nombre limpio y contexto
    pub fn find_symbol(&self, clean_name: &str, file_path: &str) -> Option<&SymbolEntry> {
        let file_basename = std::path::Path::new(file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(file_path);

        // 1. Coincidencia exacta por archivo
        for m in &self.modules {
            for f in &m.files {
                if f.file_path == file_path || f.file_name == file_path || f.file_name == file_basename {
                    for s in &f.symbols {
                        if s.clean_name() == clean_name {
                            return Some(s);
                        }
                    }
                }
            }
        }

        // 2. Coincidencia flexible global por nombre limpio en todo el proyecto
        for m in &self.modules {
            for f in &m.files {
                for s in &f.symbols {
                    if s.clean_name() == clean_name {
                        return Some(s);
                    }
                }
            }
        }

        None
    }
}

fn chrono_lite_today() -> String {
    // Generador ligero de fecha AAAA-MM-DD sin depender de chrono
    // Usamos timestamp estándar UTC o valor por defecto
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            let days = secs / 86400;
            // Aproximación simple de calendario gregoriano
            let mut y = 1970;
            let mut d_left = days;
            loop {
                let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
                let days_in_year = if leap { 366 } else { 365 };
                if d_left >= days_in_year {
                    d_left -= days_in_year;
                    y += 1;
                } else {
                    break;
                }
            }
            let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
            let days_in_months = [
                31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31
            ];
            let mut m = 1;
            for &dim in &days_in_months {
                if d_left >= dim {
                    d_left -= dim;
                    m += 1;
                } else {
                    break;
                }
            }
            let day = d_left + 1;
            format!("{:04}-{:02}-{:02}", y, m, day)
        })
        .unwrap_or_else(|_| "2026-11-11".to_string())
}
