use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "glosaric",
    about = "Navegación de código determinista y generador de glosarios duales para desarrolladores e IAs",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Inicializa un nuevo glosario escaneando el código fuente
    Init {
        /// Ruta al repositorio o código fuente
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Archivo de salida para el Glosaric (por defecto docs/GLOSARIC.md)
        #[arg(short, long)]
        out: Option<PathBuf>,

        /// Sincronizar directamente con la Bóveda Obsidian (/mnt/Vault/01_Obsidian/Glosaric/)
        #[arg(long)]
        vault: Option<Option<PathBuf>>,

        /// Nombre explícito del proyecto
        #[arg(short, long)]
        project: Option<String>,

        /// Crear una copia de respaldo (.bak) si el archivo de destino ya existía
        #[arg(long, default_value_t = true)]
        backup: bool,
    },

    /// Audita el código fuente contra el Glosaric (ideal para CI/CD y pre-commit)
    Check {
        /// Ruta al repositorio o código fuente
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Ruta al archivo Glosaric a verificar
        #[arg(short, long)]
        doc: Option<PathBuf>,

        /// Verificar contra la Bóveda Obsidian (/mnt/Vault/01_Obsidian/Glosaric/)
        #[arg(long)]
        vault: Option<Option<PathBuf>>,

        /// Falla también si existen descripciones marcadas como [Pendiente]
        #[arg(long, default_value_t = false)]
        strict: bool,

        /// Salida estructurada en JSON para herramientas de automatización e IAs
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Sincroniza nuevos símbolos del código sin sobreescribir las explicaciones existentes
    Sync {
        /// Ruta al repositorio o código fuente
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Ruta al archivo Glosaric a actualizar
        #[arg(short, long)]
        doc: Option<PathBuf>,

        /// Sincronizar directamente con la Bóveda Obsidian (/mnt/Vault/01_Obsidian/Glosaric/)
        #[arg(long)]
        vault: Option<Option<PathBuf>>,

        /// Crear copia de respaldo (.backup.md) antes de escribir
        #[arg(long, default_value_t = true)]
        backup: bool,
    },

    /// Búsqueda relámpago (< 10 ms) de funciones, structs o explicaciones en el glosario
    Query {
        /// Término de búsqueda (símbolo, tipo o palabra clave)
        term: String,

        /// Ruta al repositorio o código fuente
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Ruta específica al Glosaric
        #[arg(short, long)]
        doc: Option<PathBuf>,

        /// Buscar en la Bóveda Obsidian (/mnt/Vault/01_Obsidian/Glosaric/)
        #[arg(long)]
        vault: Option<Option<PathBuf>>,

        /// Salida en formato JSON para consumo por agentes e IAs
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Lista los lenguajes de programación soportados activamente por glosaric
    Languages,
}

/// Resuelve la ruta canónica del archivo Glosaric según los argumentos de la CLI
pub fn resolve_glosaric_path(
    repo_path: &PathBuf,
    explicit_doc: Option<&PathBuf>,
    vault_opt: Option<&Option<PathBuf>>,
    project_name: &str,
) -> PathBuf {
    if let Some(explicit) = explicit_doc {
        return explicit.clone();
    }

    if let Some(vault_inner) = vault_opt {
        if let Some(custom_vault_path) = vault_inner {
            return custom_vault_path.clone();
        } else {
            // Ruta estándar en la Bóveda Obsidian
            let vault_base = PathBuf::from("/mnt/Vault/01_Obsidian/Glosaric");
            return vault_base.join(format!("Glosaric_{}.md", project_name));
        }
    }

    // Ruta por defecto: docs/GLOSARIC.md
    repo_path.join("docs").join("GLOSARIC.md")
}

/// Extrae o deduce el nombre del proyecto a partir de la ruta o Cargo.toml
pub fn deduce_project_name(repo_path: &PathBuf, explicit_name: Option<&str>) -> String {
    if let Some(name) = explicit_name {
        return name.to_string();
    }

    // Intentar leer de Cargo.toml si existe (Rust)
    let cargo_path = repo_path.join("Cargo.toml");
    if cargo_path.exists() {
        if let Ok(content) = std::fs::read_to_string(cargo_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("name") {
                    if let Some((_, val)) = trimmed.split_once('=') {
                        let clean = val.trim().trim_matches('"').trim_matches('\'').trim();
                        if !clean.is_empty() {
                            return clean.to_string();
                        }
                    }
                }
            }
        }
    }

    // Intentar leer de pubspec.yaml si existe (Flutter / Dart)
    let pubspec_path = repo_path.join("pubspec.yaml");
    if pubspec_path.exists() {
        if let Ok(content) = std::fs::read_to_string(pubspec_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("name:") {
                    if let Some((_, val)) = trimmed.split_once(':') {
                        let clean = val.trim().trim_matches('"').trim_matches('\'').trim();
                        if !clean.is_empty() {
                            return clean.to_string();
                        }
                    }
                }
            }
        }
    }

    // Fallback: nombre de la carpeta
    repo_path
        .canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "Proyecto".to_string())
}
