---
proyecto: glosaric
tipo: Glosario de Navegación Rápida
actualizado: 2026-11-11
---

# 📖 Glosaric // glosaric — Mapa Rápido de Símbolos y Contratos

> **Misión:** Mapa de cabina y contrato vivo para localización instantánea de símbolos, structs y callbacks (< 15s con `Ctrl + F`), evitando quemar tokens en lecturas redundantes del árbol de código.  
> **Formato Canónico:** `- simbolo(params) | Explicación técnica precisa | Traducción humana / para qué sirve en la práctica`.

---

## 📂 `src/` (Punto de Entrada, CLI y Modelo de Dominio)

### `cli.rs`
- `Cli` | Parser raíz de argumentos Clap | Estructura principal que recibe y enruta los comandos de terminal.
- `Commands` | Enum con 4 subcomandos (Init, Check, Sync, Query) | Define las operaciones fundamentales soportadas por la herramienta.
- `resolve_glosaric_path(repo_path, explicit_doc, vault_opt, project_name)` | fn(& PathBuf, Option < & PathBuf >, Option < & Option < PathBuf > >, & str) -> PathBuf | Resuelve la ruta canónica a docs/GLOSARIC.md o la Bóveda Obsidian según los flags.
- `deduce_project_name(repo_path, explicit_name)` | fn(& PathBuf, Option < & str >) -> String | Extrae el nombre formal del proyecto desde Cargo.toml o el directorio raíz.

### `model.rs`
- `SymbolKind` | Enum de 8 categorías sintácticas (Fn, Struct, Enum, Trait, etc.) | Clasifica qué elemento de código representa cada fila documentada.
- `SymbolEntry` | Struct atómico con nombre, tipos, docstrings y traducción humana | Almacena una entrada individual completa del glosario en memoria.
- `to_markdown_line()` | fn(&self) -> String | Formatea el símbolo al estándar canónico de tres columnas separadas por barritas.
- `clean_name()` | fn(&self) -> String | Extrae el nombre puro sin comillas ni paréntesis para cotejar duplicados y cambios.
- `FileSection` | Contenedor de símbolos agrupados por archivo de origen | Representa una subsección `### `archivo.rs`` en el documento.
- `ModuleSection` | Contenedor de archivos agrupados por directorio relativo | Representa un encabezado de módulo `## 📂 `src/modulo/``.
- `GlosaricDoc` | Documento raíz en memoria con frontmatter, misión y módulos | Estructura central que se lee, audita, sincroniza y guarda en disco.
- `new(project_name)` | fn(& str) -> Self | Constructor que inicializa un GlosaricDoc nuevo con metadatos y fecha actual.
- `total_symbols()` | fn(&self) -> usize | Conteo acumulado de todos los símbolos registrados en el documento.
- `find_symbol(clean_name, file_path)` | fn(&self, & str, & str) -> Option < & SymbolEntry > | Búsqueda por archivo y fallback global por nombre para no sobreescribir traducciones.

## 📂 `src/engine/` (Motores de Escaneo, Fusión No Destructiva, Auditoría y Búsqueda)

### `auditor.rs`
- `AuditReport` | Struct con listas de huérfanos, obsoletos y pendientes | Entrega el diagnóstico detallado de deuda técnica para CI/CD y pre-commit.
- `is_clean()` | fn(&self) -> bool | Evalúa si el código fuente está 100% cubierto por el Glosario sin omisiones.
- `audit_project(scanned_modules, glosaric_path, project_name)` | fn(& [ModuleSection], & Path, & str) -> Result < AuditReport > | Compara el AST del código contra el Glosario y reporta discrepancias.

### `merger.rs`
- `MergeOptions` | Struct de configuración con flags de respaldo y nombre | Parámetros que controlan la sincronización y la política de backup.
- `MergeResult` | Struct con estadísticas de sincronización (totales, nuevos, preservados) | Informa al usuario qué cambios se aplicaron tras el proceso de merge.
- `sync_and_save(scanned_modules, target_path, options)` | fn(Vec < ModuleSection >, & Path, MergeOptions) -> Result < MergeResult > | Fusión inteligente no destructiva que anexa símbolos nuevos preservando traducciones.

### `query.rs`
- `QueryResult` | Struct serializable con término buscado y coincidencias | Almacena los resultados de búsqueda para salida en terminal o JSON para IAs.
- `search_glosaric(glosaric_path, term, project_name)` | fn(& Path, & str, & str) -> Result < QueryResult > | Localiza símbolos por nombre, firma o traducción humana en menos de 3 ms.

### `scanner.rs`
- `CodeScanner` | Escáner recursivo del filesystem con filtro de exclusión | Recorre el árbol de código ignorando target, .git y node_modules.
- `new()` | fn() -> Self | Constructor de CodeScanner inicializado con los parsers activos.
- `scan_project(repo_root)` | fn(&self, & Path) -> Result < Vec < ModuleSection > > | Escanea recursivamente el proyecto y organiza los símbolos por módulo y archivo.

## 📂 `src/format/` (Serialización y Deserialización Markdown Canónico)

### `markdown_parser.rs`
- `parse_markdown(content, default_project_name)` | fn(& str, & str) -> Result < GlosaricDoc > | Parsea un archivo Glosaric existente respetando descripciones previas y subtítulos.

### `markdown_writer.rs`
- `render_markdown(doc)` | fn(& GlosaricDoc) -> String | Serializa un GlosaricDoc a texto Markdown estándar canónico con frontmatter.

## 📂 `src/parser/` (Extractores Sintácticos AST Multi-Lenguaje)

### `mod.rs`
- `SourceParser` | Trait abstracto para extractores de sintaxis de código | Interfaz común para soportar Rust, Dart, TypeScript, Python, etc.
- `default_parsers()` | fn() -> Vec < Box < dyn SourceParser > > | Fábrica de parsers activos disponibles en el binario.

### `rust_syn.rs`
- `RustSynParser` | Extractor de sintaxis Rust basado en syn y quote | Parsea archivos .rs extrayendo visibilidades públicas y docstrings.
- `new()` | fn() -> Self | Constructor de RustSynParser.
