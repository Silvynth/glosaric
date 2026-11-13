use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use regex::Regex;

use crate::model::{SymbolEntry, SymbolKind};
use super::SourceParser;

pub struct SwiftParser {
    fn_re: Regex,
    struct_class_re: Regex,
    protocol_re: Regex,
    enum_re: Regex,
}

impl SwiftParser {
    pub fn new() -> Self {
        Self {
            // [public|open|static|mutating|override] func nombre(params) [async] [throws] [-> Retorno]
            fn_re: Regex::new(
                r"^\s*(?:(?:public|open|static|mutating|override|final)\s+)*func\s+([A-Za-z0-9_]+)\s*(?:<[^>]+>)?\s*\((.*?)\)(?:\s*(?:async|throws)\s*)*(?:\s*->\s*([^;{]+))?"
            ).unwrap(),

            // [public|open|final] class|struct|actor Nombre [: Base]
            struct_class_re: Regex::new(
                r"^\s*(?:(?:public|open|final)\s+)*(class|struct|actor)\s+([A-Za-z0-9_]+)(?:\s*<[^>]+>)?(?:\s*:\s*([A-Za-z0-9_<>, ]+))?"
            ).unwrap(),

            // [public] protocol Nombre [: Base]
            protocol_re: Regex::new(
                r"^\s*(?:public\s+)?protocol\s+([A-Za-z0-9_]+)(?:\s*:\s*([A-Za-z0-9_<>, ]+))?"
            ).unwrap(),

            // [public] enum Nombre [: Base]
            enum_re: Regex::new(
                r"^\s*(?:public\s+)?enum\s+([A-Za-z0-9_]+)(?:\s*:\s*([A-Za-z0-9_<>, ]+))?"
            ).unwrap(),
        }
    }

    pub fn parse_content(&self, content: &str, relative_path: &str) -> Result<Vec<SymbolEntry>> {
        let mut symbols = Vec::new();
        let mut doc_buffer: Vec<String> = Vec::new();
        let mut in_multiline_doc = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Bloques /** ... */
            if trimmed.starts_with("/**") {
                doc_buffer.clear();
                in_multiline_doc = true;
                let rest = trimmed.trim_start_matches("/**").trim();
                if let Some(clean_end) = rest.strip_suffix("*/") {
                    let single = clean_end.trim().trim_start_matches('*').trim();
                    if !single.is_empty() {
                        doc_buffer.push(single.to_string());
                    }
                    in_multiline_doc = false;
                } else if !rest.is_empty() {
                    let clean = rest.trim_start_matches('*').trim();
                    if !clean.is_empty() {
                        doc_buffer.push(clean.to_string());
                    }
                }
                continue;
            }

            if in_multiline_doc {
                if let Some(clean_end) = trimmed.strip_suffix("*/") {
                    let clean = clean_end.trim().trim_start_matches('*').trim();
                    if !clean.is_empty() && !clean.starts_with('@') {
                        doc_buffer.push(clean.to_string());
                    }
                    in_multiline_doc = false;
                } else {
                    let clean = trimmed.trim_start_matches('*').trim();
                    if !clean.is_empty() && !clean.starts_with('@') {
                        doc_buffer.push(clean.to_string());
                    }
                }
                continue;
            }

            // Documentación Swift /// o //
            if trimmed.starts_with("///") || trimmed.starts_with("//") {
                let clean = trimmed.trim_start_matches('/').trim();
                if !clean.is_empty() && !clean.starts_with('@') {
                    doc_buffer.push(clean.to_string());
                }
                continue;
            }

            if trimmed.is_empty() || trimmed.starts_with("import ") {
                if trimmed.starts_with("import ") {
                    doc_buffer.clear();
                }
                continue;
            }

            // Ignorar elementos privados o file-private
            if trimmed.starts_with("private ") || trimmed.starts_with("fileprivate ") {
                doc_buffer.clear();
                continue;
            }

            let doc = if doc_buffer.is_empty() {
                None
            } else {
                Some(doc_buffer.join(" "))
            };

            // 1. Struct, Class, Actor
            if let Some(caps) = self.struct_class_re.captures(trimmed) {
                let kind_str = caps.get(1).map(|m| m.as_str()).unwrap_or("class");
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let inheritance = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                let sig_tech = if inheritance.is_empty() {
                    format!("{kind_str} {name}")
                } else {
                    format!("{kind_str} {name} : {inheritance}")
                };

                symbols.push(SymbolEntry {
                    name: name.to_string(),
                    kind: SymbolKind::Struct,
                    file_path: relative_path.to_string(),
                    signature_tech: sig_tech,
                    human_desc: None,
                    doc_comment: doc,
                });
                doc_buffer.clear();
                continue;
            }

            // 2. Protocols
            if let Some(caps) = self.protocol_re.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let inheritance = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");

                let sig_tech = if inheritance.is_empty() {
                    format!("protocol {name}")
                } else {
                    format!("protocol {name} : {inheritance}")
                };

                symbols.push(SymbolEntry {
                    name: name.to_string(),
                    kind: SymbolKind::Trait,
                    file_path: relative_path.to_string(),
                    signature_tech: sig_tech,
                    human_desc: None,
                    doc_comment: doc,
                });
                doc_buffer.clear();
                continue;
            }

            // 3. Enums
            if let Some(caps) = self.enum_re.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                symbols.push(SymbolEntry {
                    name: name.to_string(),
                    kind: SymbolKind::Enum,
                    file_path: relative_path.to_string(),
                    signature_tech: format!("enum {name}"),
                    human_desc: None,
                    doc_comment: doc,
                });
                doc_buffer.clear();
                continue;
            }

            // 4. Funciones
            if let Some(caps) = self.fn_re.captures(trimmed) {
                let fn_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let params = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                let ret = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                let sig_tech = if ret.is_empty() {
                    format!("func {fn_name}({params})")
                } else {
                    format!("func {fn_name}({params}) -> {ret}")
                };

                symbols.push(SymbolEntry {
                    name: format!("{fn_name}()"),
                    kind: SymbolKind::Function,
                    file_path: relative_path.to_string(),
                    signature_tech: sig_tech,
                    human_desc: None,
                    doc_comment: doc,
                });
                doc_buffer.clear();
                continue;
            }

            doc_buffer.clear();
        }

        Ok(symbols)
    }
}

impl Default for SwiftParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceParser for SwiftParser {
    fn can_parse(&self, path: &Path) -> bool {
        path.extension().and_then(|s| s.to_str()) == Some("swift")
    }

    fn parse_file(&self, file_path: &Path, repo_root: &Path) -> Result<Vec<SymbolEntry>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("No se pudo leer archivo Swift {}", file_path.display()))?;

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
    fn test_swift_parser_extracts_symbols() {
        let parser = SwiftParser::new();
        let code = r#"
import Flutter
import UIKit

/// Plugin oficial para iOS en Licarayen Connect.
public class IosBridgePlugin: NSObject, FlutterPlugin {
    public static func register(with registrar: FlutterPluginRegistrar) {}
}

public protocol TelemetryReceiver {
    func onReceive(data: [String: Any])
}

public enum ConnectionStatus: String {
    case active
    case idle
}

public struct DeviceConfig {
    let timeout: Int
}

/// Registra el canal de métodos nativo de iOS.
func setupMethodChannel(messenger: FlutterBinaryMessenger) -> FlutterMethodChannel {
    return FlutterMethodChannel(name: "com.licarayen/bridge", binaryMessenger: messenger)
}

private func privateCryptoHelper() {}
"#;

        let symbols = parser.parse_content(code, "ios/IosBridgePlugin.swift").unwrap();

        assert_eq!(symbols.len(), 7);
        assert_eq!(symbols[0].name, "IosBridgePlugin");
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
        assert_eq!(symbols[0].doc_comment.as_deref(), Some("Plugin oficial para iOS en Licarayen Connect."));

        assert_eq!(symbols[1].name, "register()");
        assert_eq!(symbols[1].kind, SymbolKind::Function);

        assert_eq!(symbols[2].name, "TelemetryReceiver");
        assert_eq!(symbols[2].kind, SymbolKind::Trait);

        assert_eq!(symbols[3].name, "onReceive()");
        assert_eq!(symbols[3].kind, SymbolKind::Function);

        assert_eq!(symbols[4].name, "ConnectionStatus");
        assert_eq!(symbols[4].kind, SymbolKind::Enum);

        assert_eq!(symbols[5].name, "DeviceConfig");
        assert_eq!(symbols[5].kind, SymbolKind::Struct);

        assert_eq!(symbols[6].name, "setupMethodChannel()");
        assert_eq!(symbols[6].kind, SymbolKind::Function);
        assert_eq!(symbols[6].doc_comment.as_deref(), Some("Registra el canal de métodos nativo de iOS."));
    }
}
