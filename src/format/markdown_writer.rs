use crate::model::GlosaricDoc;

pub fn render_markdown(doc: &GlosaricDoc) -> String {
    let mut out = String::new();

    // Frontmatter YAML
    out.push_str("---\n");
    out.push_str(&format!("proyecto: {}\n", doc.project_name));
    out.push_str(&format!("tipo: {}\n", doc.doc_type));
    out.push_str(&format!("actualizado: {}\n", doc.updated_at));
    out.push_str("---\n\n");

    // Encabezado
    out.push_str(&format!("# {}\n\n", doc.title));
    out.push_str(&format!("> **Misión:** {}  \n", doc.mission));
    out.push_str("> **Formato Canónico:** `- simbolo(params) | Explicación técnica precisa | Traducción humana / para qué sirve en la práctica`.\n\n");
    out.push_str("---\n\n");

    // Módulos
    for module in &doc.modules {
        if module.files.is_empty() {
            continue;
        }

        let desc = match &module.description {
            Some(d) if !d.trim().is_empty() => format!(" {}", d.trim()),
            _ => String::new(),
        };

        out.push_str(&format!("## 📂 `{}`{}\n\n", module.module_path, desc));

        for file in &module.files {
            if file.symbols.is_empty() {
                continue;
            }

            out.push_str(&format!("### `{}`\n", file.file_name));
            for sym in &file.symbols {
                out.push_str(&sym.to_markdown_line());
                out.push('\n');
            }
            out.push('\n');
        }
    }

    out
}
