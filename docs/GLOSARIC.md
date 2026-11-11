---
proyecto: glosaric
tipo: Glosario de Navegación Rápida
actualizado: 2026-11-11
---

# 📖 Glosaric // glosaric — Mapa Rápido de Símbolos y Contratos

> **Misión:** Mapa de cabina y contrato vivo para localización instantánea de símbolos, structs y callbacks (< 15s con `Ctrl + F`), evitando quemar tokens en lecturas redundantes del árbol de código.  
> **Formato Canónico:** `- simbolo(params) | Explicación técnica precisa | Traducción humana / para qué sirve en la práctica`.

---

## 📂 `src/`

### `cli.rs`
- `Cli` | Struct Cli | [Pendiente de documentar]
- `Commands` | Enum de 4 variantes | [Pendiente de documentar]
- `resolve_glosaric_path(repo_path, explicit_doc, vault_opt, project_name)` | fn(& PathBuf, Option < & PathBuf >, Option < & Option < PathBuf > >, & str) -> PathBuf | Resuelve la ruta canónica del archivo Glosaric según los argumentos de la CLI
- `deduce_project_name(repo_path, explicit_name)` | fn(& PathBuf, Option < & str >) -> String | Extrae o deduce el nombre del proyecto a partir de la ruta o Cargo.toml

### `model.rs`
- `SymbolKind` | Enum de 8 variantes | [Pendiente de documentar]
- `SymbolEntry` | Struct SymbolEntry | [Pendiente de documentar]
- `to_markdown_line()` | fn(&self) -> String | Formatea la línea al estándar canónico: `- `simbolo(params)` | Explicación técnica precisa | Traducción humana`
- `clean_name()` | fn(&self) -> String | Nombre limpio sin comillas ni paréntesis para comparaciones
- `FileSection` | Struct FileSection | [Pendiente de documentar]
- `ModuleSection` | Struct ModuleSection | [Pendiente de documentar]
- `GlosaricDoc` | Struct GlosaricDoc | [Pendiente de documentar]
- `new(project_name)` | fn(& str) -> Self | [Pendiente de documentar]
- `total_symbols()` | fn(&self) -> usize | Cuenta total de símbolos registrados en todo el documento
- `find_symbol(clean_name, file_path)` | fn(&self, & str, & str) -> Option < & SymbolEntry > | Busca un símbolo existente por nombre limpio y archivo

## 📂 `src/engine/`

### `auditor.rs`
- `AuditReport` | Struct AuditReport | [Pendiente de documentar]
- `is_clean()` | fn(&self) -> bool | [Pendiente de documentar]
- `audit_project(scanned_modules, glosaric_path, project_name)` | fn(& [ModuleSection], & Path, & str) -> Result < AuditReport > | [Pendiente de documentar]

### `merger.rs`
- `MergeOptions` | Struct MergeOptions | [Pendiente de documentar]
- `MergeResult` | Struct MergeResult | [Pendiente de documentar]
- `sync_and_save(scanned_modules, target_path, options)` | fn(Vec < ModuleSection >, & Path, MergeOptions) -> Result < MergeResult > | [Pendiente de documentar]

### `query.rs`
- `QueryResult` | Struct QueryResult | [Pendiente de documentar]
- `search_glosaric(glosaric_path, term, project_name)` | fn(& Path, & str, & str) -> Result < QueryResult > | [Pendiente de documentar]

### `scanner.rs`
- `CodeScanner` | Struct CodeScanner | [Pendiente de documentar]
- `new()` | fn() -> Self | [Pendiente de documentar]
- `scan_project(repo_root)` | fn(&self, & Path) -> Result < Vec < ModuleSection > > | Escanea recursivamente el proyecto y organiza los símbolos por módulo y archivo

## 📂 `src/format/`

### `markdown_parser.rs`
- `parse_markdown(content, default_project_name)` | fn(& str, & str) -> Result < GlosaricDoc > | [Pendiente de documentar]

### `markdown_writer.rs`
- `render_markdown(doc)` | fn(& GlosaricDoc) -> String | [Pendiente de documentar]

## 📂 `src/parser/`

### `mod.rs`
- `SourceParser` | Trait SourceParser | [Pendiente de documentar]
- `default_parsers()` | fn() -> Vec < Box < dyn SourceParser > > | Obtiene todos los parsers disponibles en el sistema

### `rust_syn.rs`
- `RustSynParser` | Struct RustSynParser | [Pendiente de documentar]
- `new()` | fn() -> Self | [Pendiente de documentar]

