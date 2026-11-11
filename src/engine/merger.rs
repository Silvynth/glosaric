use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

use crate::format::markdown_parser::parse_markdown;
use crate::format::markdown_writer::render_markdown;
use crate::model::{FileSection, GlosaricDoc, ModuleSection};

pub struct MergeOptions {
    pub make_backup: bool,
    pub project_name: String,
}

pub struct MergeResult {
    pub total_symbols: usize,
    pub existing_symbols: usize,
    pub new_symbols: usize,
    pub backup_created: Option<String>,
}

pub fn sync_and_save(
    scanned_modules: Vec<ModuleSection>,
    target_path: &Path,
    options: MergeOptions,
) -> Result<MergeResult> {
    let mut backup_created = None;

    // 1. Leer Glosaric previo si existe
    let mut base_doc = if target_path.exists() {
        let content = fs::read_to_string(target_path)
            .with_context(|| format!("No se pudo leer el archivo existente {}", target_path.display()))?;

        // Si se pidió backup, respaldamos antes de tocar nada
        if options.make_backup {
            let backup_path = target_path.with_extension("backup.md");
            fs::copy(target_path, &backup_path).with_context(|| {
                format!("No se pudo crear backup en {}", backup_path.display())
            })?;
            backup_created = Some(backup_path.to_string_lossy().to_string());
        }

        parse_markdown(&content, &options.project_name)?
    } else {
        GlosaricDoc::new(&options.project_name)
    };

    let mut existing_count = 0;
    let mut new_count = 0;

    // 2. Fusionar módulos escaneados dentro del documento base
    for scanned_mod in scanned_modules {
        let module_idx = match base_doc
            .modules
            .iter()
            .position(|m| m.module_path == scanned_mod.module_path)
        {
            Some(idx) => idx,
            None => {
                base_doc.modules.push(ModuleSection {
                    module_path: scanned_mod.module_path.clone(),
                    description: scanned_mod.description.clone(),
                    files: Vec::new(),
                });
                base_doc.modules.len() - 1
            }
        };

        let target_module = &mut base_doc.modules[module_idx];

        for scanned_file in scanned_mod.files {
            let file_idx = match target_module
                .files
                .iter()
                .position(|f| f.file_name == scanned_file.file_name)
            {
                Some(idx) => idx,
                None => {
                    target_module.files.push(FileSection {
                        file_name: scanned_file.file_name.clone(),
                        file_path: scanned_file.file_path.clone(),
                        symbols: Vec::new(),
                    });
                    target_module.files.len() - 1
                }
            };

            let target_file = &mut target_module.files[file_idx];

            for scanned_sym in scanned_file.symbols {
                // Buscar si ya existía en el glosario por nombre limpio
                let clean = scanned_sym.clean_name();
                if let Some(existing_sym) = target_file
                    .symbols
                    .iter_mut()
                    .find(|s| s.clean_name() == clean)
                {
                    // Preservar la traducción humana existente
                    existing_count += 1;
                    // Actualizar firma técnica si el usuario no tenía una manual elaborada
                    if existing_sym.signature_tech.is_empty() {
                        existing_sym.signature_tech = scanned_sym.signature_tech;
                    }
                } else {
                    // Símbolo nuevo detectado
                    new_count += 1;
                    target_file.symbols.push(scanned_sym);
                }
            }
        }
    }

    // Actualizar fecha de modificación
    base_doc.updated_at = crate::model::GlosaricDoc::new("").updated_at;

    // 3. Escribir resultado a disco
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let markdown_content = render_markdown(&base_doc);
    fs::write(target_path, markdown_content)?;

    Ok(MergeResult {
        total_symbols: base_doc.total_symbols(),
        existing_symbols: existing_count,
        new_symbols: new_count,
        backup_created,
    })
}
