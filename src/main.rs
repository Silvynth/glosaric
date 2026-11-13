mod cli;
mod engine;
mod format;
mod model;
mod parser;

use std::path::PathBuf;
use std::time::Instant;
use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use cli::{deduce_project_name, resolve_glosaric_path, Cli, Commands};
use engine::auditor::audit_project;
use engine::merger::{sync_and_save, MergeOptions};
use engine::query::search_glosaric;
use engine::scanner::CodeScanner;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let scanner = CodeScanner::new();

    match cli.command {
        Commands::Init {
            path,
            out,
            vault,
            project,
            backup,
        } => {
            let start = Instant::now();
            let project_name = deduce_project_name(&path, project.as_deref());
            let target_path = resolve_target_path(&path, out, vault.as_ref(), &project_name);

            println!(
                "{} {} (Proyecto: {})",
                "🚀 [glosaric]".cyan().bold(),
                "Iniciando escaneo de código fuente...".white(),
                project_name.green().bold()
            );

            let scan_res = scanner.scan_project(&path)?;
            let scanned_modules = scan_res.modules;

            let merge_opts = MergeOptions {
                make_backup: backup,
                project_name: project_name.clone(),
            };

            let res = sync_and_save(scanned_modules, &target_path, merge_opts)?;

            println!(
                "{} Glosario generado con éxito en {}",
                "✅ [OK]".green().bold(),
                target_path.display().to_string().cyan()
            );
            println!(
                "   📊 Símbolos totales: {} (Nuevos: {}, Existentes: {}) en {:.2} ms",
                res.total_symbols.to_string().yellow().bold(),
                res.new_symbols.to_string().green(),
                res.existing_symbols.to_string().blue(),
                start.elapsed().as_secs_f64() * 1000.0
            );

            if let Some(bak) = res.backup_created {
                println!("   💾 Copia de seguridad guardada en: {}", bak.dimmed());
            }

            // Advertencia pedagógica si se detectaron lenguajes aún no soportados
            if !scan_res.unsupported_languages.is_empty() {
                println!(
                    "\n{} Se detectaron archivos en lenguajes aún no soportados en este proyecto:",
                    "ℹ️  [Aviso]".yellow().bold()
                );
                for (lang, count) in &scan_res.unsupported_languages {
                    println!("   {} {} ({} archivos)", "-".yellow(), lang, count.to_string().cyan());
                }
                println!(
                    "   💡 Lenguajes soportados actualmente: {} | {} | {} | {} | {} | {} | {} | {}",
                    "Rust (.rs)".green(),
                    "Dart (.dart)".green(),
                    "Python (.py)".green(),
                    "TypeScript / JS (.ts, .js)".green(),
                    "Go (.go)".green(),
                    "C / C++ (.c, .h, .cpp)".green(),
                    "Kotlin (.kt)".green(),
                    "Swift (.swift)".green()
                );
            }
        }

        Commands::Sync {
            path,
            doc,
            vault,
            backup,
        } => {
            let start = Instant::now();
            let project_name = deduce_project_name(&path, None);
            let target_path = resolve_glosaric_path(&path, doc.as_ref(), vault.as_ref(), &project_name);

            println!(
                "{} Sincronizando símbolos para {} en {}...",
                "🔄 [glosaric]".blue().bold(),
                project_name.green().bold(),
                target_path.display().to_string().cyan()
            );

            let scan_res = scanner.scan_project(&path)?;
            let scanned_modules = scan_res.modules;
            let merge_opts = MergeOptions {
                make_backup: backup,
                project_name,
            };

            let res = sync_and_save(scanned_modules, &target_path, merge_opts)?;

            println!(
                "{} Sincronización completada sin pérdida de datos en {:.2} ms",
                "✅ [OK]".green().bold(),
                start.elapsed().as_secs_f64() * 1000.0
            );
            println!(
                "   📊 Total en Glosaric: {} | Nuevos integrados: {} | Existentes preservados: {}",
                res.total_symbols.to_string().yellow().bold(),
                res.new_symbols.to_string().green().bold(),
                res.existing_symbols.to_string().blue()
            );

            if let Some(bak) = res.backup_created {
                println!("   💾 Backup creado: {}", bak.dimmed());
            }

            if !scan_res.unsupported_languages.is_empty() {
                println!(
                    "\n{} Se detectaron archivos en lenguajes aún no soportados en este proyecto:",
                    "ℹ️  [Aviso]".yellow().bold()
                );
                for (lang, count) in &scan_res.unsupported_languages {
                    println!("   {} {} ({} archivos)", "-".yellow(), lang, count.to_string().cyan());
                }
            }
        }

        Commands::Check {
            path,
            doc,
            vault,
            strict,
            json,
        } => {
            let project_name = deduce_project_name(&path, None);
            let target_path = resolve_glosaric_path(&path, doc.as_ref(), vault.as_ref(), &project_name);

            let scan_res = scanner.scan_project(&path)?;
            let scanned_modules = scan_res.modules;
            let report = audit_project(&scanned_modules, &target_path, &project_name)?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "project": project_name,
                        "glosaric_path": target_path.display().to_string(),
                        "is_clean": report.is_clean(),
                        "total_code_symbols": report.total_code_symbols,
                        "total_doc_symbols": report.total_doc_symbols,
                        "missing_in_doc": report.missing_in_doc,
                        "obsolete_in_doc": report.obsolete_in_doc,
                        "pending_descriptions": report.pending_descriptions,
                    }))?
                );
            } else {
                println!(
                    "{} Auditando {} contra {}",
                    "🔍 [glosaric]".cyan().bold(),
                    path.display().to_string().bold(),
                    target_path.display().to_string().cyan()
                );

                if !report.missing_in_doc.is_empty() {
                    println!(
                        "\n{} Se encontraron {} símbolos públicos en código sin registrar en Glosaric:",
                        "❌ ERROR:".red().bold(),
                        report.missing_in_doc.len().to_string().red().bold()
                    );
                    for item in &report.missing_in_doc {
                        println!("   {} {}", "-".red(), item);
                    }
                }

                if !report.obsolete_in_doc.is_empty() {
                    println!(
                        "\n{} {} símbolos en Glosaric ya no existen en el código fuente:",
                        "⚠️  ADVERTENCIA:".yellow().bold(),
                        report.obsolete_in_doc.len()
                    );
                    for item in &report.obsolete_in_doc {
                        println!("   {} {}", "-".yellow(), item);
                    }
                }

                if strict && !report.pending_descriptions.is_empty() {
                    println!(
                        "\n{} Hay {} símbolos con descripciones marcadas como [Pendiente]:",
                        "❌ MODO STRICT:".red().bold(),
                        report.pending_descriptions.len()
                    );
                    for item in &report.pending_descriptions {
                        println!("   {} {}", "-".red(), item);
                    }
                }

                println!("\n{}", "──────────────────────────────────────────────".dimmed());
                println!(
                    "📊 Símbolos en código: {} | Símbolos en Glosaric: {}",
                    report.total_code_symbols.to_string().cyan(),
                    report.total_doc_symbols.to_string().cyan()
                );

                if report.is_clean() && (!strict || report.pending_descriptions.is_empty()) {
                    println!(
                        "{} Glosario 100% sincronizado. Cero deuda técnica.",
                        "✨ ÉXITO:".green().bold()
                    );
                } else {
                    println!(
                        "{} Se detectó deuda documental. Ejecuta `glosaric sync` para actualizar.",
                        "🚨 FALLO:".red().bold()
                    );
                    std::process::exit(1);
                }
            }
        }

        Commands::Query {
            term,
            path,
            doc,
            vault,
            json,
        } => {
            let project_name = deduce_project_name(&path, None);
            let target_path = resolve_glosaric_path(&path, doc.as_ref(), vault.as_ref(), &project_name);

            let res = search_glosaric(&target_path, &term, &project_name)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&res)?);
            } else {
                println!(
                    "{} Búsqueda de '{}' en {} ({} coincidencias):",
                    "🔎 [glosaric]".cyan().bold(),
                    term.yellow().bold(),
                    target_path.file_name().unwrap_or_default().to_string_lossy().cyan(),
                    res.matches_count.to_string().green().bold()
                );

                for item in &res.results {
                    println!("{}", item.to_markdown_line());
                }
            }
        }

        Commands::Languages => {
            println!("{}", "🌐 Lenguajes soportados activamente por glosaric:".cyan().bold());
            println!("   {} 🦀 Rust (.rs)             - Extracción AST completa con syn (pub fn, struct, enum, docstrings)", "✓".green().bold());
            println!("   {} 🎯 Dart / Flutter (.dart)   - Extracción de clases, widgets, métodos públicos y comentarios //", "✓".green().bold());
            println!("   {} 🐍 Python (.py)            - Extracción de clases, funciones y docstrings multilínea \"\"\"", "✓".green().bold());
            println!("   {} 🔷 TypeScript (.ts, .tsx)   - Extracción de funciones, interfaces, clases, types, enums y JSDoc", "✓".green().bold());
            println!("   {} 🟨 JavaScript (.js, .jsx)   - Extracción de funciones exportadas, arrow fns y clases", "✓".green().bold());
            println!("   {} 🐹 Go (.go)                - Extracción de funciones, métodos, structs e interfaces exportadas", "✓".green().bold());
            println!("   {} ⚡ C / C++ (.c, .h, .cpp)   - Extracción de funciones públicas, structs, enums y macros #define", "✓".green().bold());
            println!("   {} 📱 Kotlin (.kt, .kts)       - Extracción de clases, interfaces, data classes, fun y KDoc", "✓".green().bold());
            println!("   {} 🍎 Swift (.swift)          - Extracción de clases, structs, protocols, enums y docstrings ///", "✓".green().bold());
            println!("\n{}", "🚀 Próximamente (Roadmap v0.2 / v0.3):".dimmed());
            println!("   - Slint UI (.slint)");
            println!("   - C# (.cs)");
            println!("   - Java (.java)");
        }
    }

    Ok(())
}

fn resolve_target_path(
    repo_path: &PathBuf,
    explicit_out: Option<PathBuf>,
    vault_opt: Option<&Option<PathBuf>>,
    project_name: &str,
) -> PathBuf {
    if let Some(out) = explicit_out {
        return out;
    }

    if let Some(vault_inner) = vault_opt {
        if let Some(custom_vault_path) = vault_inner {
            return custom_vault_path.clone();
        } else {
            let vault_base = PathBuf::from("/mnt/Vault/01_Obsidian/Glosaric");
            return vault_base.join(format!("Glosaric_{}.md", project_name));
        }
    }

    repo_path.join("docs").join("GLOSARIC.md")
}
