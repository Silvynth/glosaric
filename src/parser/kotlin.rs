use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use regex::Regex;

use crate::model::{SymbolEntry, SymbolKind};
use super::SourceParser;

pub struct KotlinParser {
    fn_re: Regex,
    class_re: Regex,
    interface_re: Regex,
    object_re: Regex,
    enum_re: Regex,
    typealias_re: Regex,
}

impl KotlinParser {
    pub fn new() -> Self {
        Self {
            // [suspend] [override] fun [ <T> ] [Receptor.]nombre(params) [: Retorno]
            fn_re: Regex::new(
                r"^\s*(?:(?:public|open|override|suspend|inline|tailrec|operator|infix)\s+)*fun\s+(?:<[^>]+>\s+)?([A-Za-z0-9_]+(?:\.[A-Za-z0-9_]+)?)\s*\((.*?)\)(?:\s*:\s*([^;{=]+))?"
            ).unwrap(),

            // [data|sealed|open|abstract|value|annotation] class Nombre [ <T> ] [ (constructor) ] [: Base]
            class_re: Regex::new(
                r"^\s*(?:(?:public|open|data|sealed|abstract|inner|value|annotation)\s+)*class\s+([A-Za-z0-9_]+)(?:\s*<[^>]+>)?(?:\s*\([^)]*\))?(?:\s*:\s*([A-Za-z0-9_<>, ()]+))?"
            ).unwrap(),

            // [sealed] interface Nombre [ <T> ] [: Base]
            interface_re: Regex::new(
                r"^\s*(?:(?:public|sealed|fun)\s+)*interface\s+([A-Za-z0-9_]+)(?:\s*<[^>]+>)?(?:\s*:\s*([A-Za-z0-9_<>, ()]+))?"
            ).unwrap(),

            // [data] [companion] object [Nombre] [: Base]
            object_re: Regex::new(
                r"^\s*(?:(?:public|data)\s+)*(?:companion\s+)?object(?:\s+([A-Za-z0-9_]+))?"
            ).unwrap(),

            // enum class Nombre
            enum_re: Regex::new(
                r"^\s*(?:public\s+)?enum\s+class\s+([A-Za-z0-9_]+)"
            ).unwrap(),

            // typealias Nombre = Destino
            typealias_re: Regex::new(
                r"^\s*(?:public\s+)?typealias\s+([A-Za-z0-9_]+)(?:\s*<[^>]+>)?\s*=\s*([^;{\n]+)"
            ).unwrap(),
        }
    }

    pub fn parse_content(&self, content: &str, relative_path: &str) -> Result<Vec<SymbolEntry>> {
        let mut symbols = Vec::new();
        let mut doc_buffer: Vec<String> = Vec::new();
        let mut in_kdoc = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Bloques KDoc /** ... */
            if trimmed.starts_with("/**") {
                doc_buffer.clear();
                in_kdoc = true;
                let rest = trimmed.trim_start_matches("/**").trim();
                if let Some(clean_end) = rest.strip_suffix("*/") {
                    let single = clean_end.trim().trim_start_matches('*').trim();
                    if !single.is_empty() {
                        doc_buffer.push(single.to_string());
                    }
                    in_kdoc = false;
                } else if !rest.is_empty() {
                    let clean = rest.trim_start_matches('*').trim();
                    if !clean.is_empty() {
                        doc_buffer.push(clean.to_string());
                    }
                }
                continue;
            }

            if in_kdoc {
                if let Some(clean_end) = trimmed.strip_suffix("*/") {
                    let clean = clean_end.trim().trim_start_matches('*').trim();
                    if !clean.is_empty() && !clean.starts_with('@') {
                        doc_buffer.push(clean.to_string());
                    }
                    in_kdoc = false;
                } else {
                    let clean = trimmed.trim_start_matches('*').trim();
                    if !clean.is_empty() && !clean.starts_with('@') {
                        doc_buffer.push(clean.to_string());
                    }
                }
                continue;
            }

            // Comentarios de una sola línea //
            if trimmed.starts_with("//") {
                let clean = trimmed.trim_start_matches('/').trim();
                if !clean.is_empty() && !clean.starts_with('@') {
                    doc_buffer.push(clean.to_string());
                }
                continue;
            }

            if trimmed.is_empty() || trimmed.starts_with("package ") || trimmed.starts_with("import ") {
                if trimmed.starts_with("package ") || trimmed.starts_with("import ") {
                    doc_buffer.clear();
                }
                continue;
            }

            // Ignorar elementos explícitamente privados o internos
            if trimmed.starts_with("private ") || trimmed.starts_with("internal ") || trimmed.starts_with("protected ") {
                doc_buffer.clear();
                continue;
            }

            let doc = if doc_buffer.is_empty() {
                None
            } else {
                Some(doc_buffer.join(" "))
            };

            // 1. Enum class
            if let Some(caps) = self.enum_re.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                symbols.push(SymbolEntry {
                    name: name.to_string(),
                    kind: SymbolKind::Enum,
                    file_path: relative_path.to_string(),
                    signature_tech: format!("enum class {name}"),
                    human_desc: None,
                    doc_comment: doc,
                });
                doc_buffer.clear();
                continue;
            }

            // 2. Classes (data, sealed, open, abstract)
            if let Some(caps) = self.class_re.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let inheritance = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");

                let sig_tech = if inheritance.is_empty() {
                    format!("class {name}")
                } else {
                    format!("class {name} : {inheritance}")
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

            // 3. Interfaces
            if let Some(caps) = self.interface_re.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let inheritance = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");

                let sig_tech = if inheritance.is_empty() {
                    format!("interface {name}")
                } else {
                    format!("interface {name} : {inheritance}")
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

            // 4. Objects (y companion objects)
            if let Some(caps) = self.object_re.captures(trimmed) {
                let raw_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let name = if raw_name.is_empty() {
                    if trimmed.contains("companion object") {
                        "Companion"
                    } else {
                        raw_name
                    }
                } else {
                    raw_name
                };

                if !name.is_empty() {
                    symbols.push(SymbolEntry {
                        name: name.to_string(),
                        kind: SymbolKind::Struct,
                        file_path: relative_path.to_string(),
                        signature_tech: format!("object {name}"),
                        human_desc: None,
                        doc_comment: doc,
                    });
                    doc_buffer.clear();
                    continue;
                }
            }

            // 5. Typealiases
            if let Some(caps) = self.typealias_re.captures(trimmed) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let target = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                symbols.push(SymbolEntry {
                    name: name.to_string(),
                    kind: SymbolKind::TypeAlias,
                    file_path: relative_path.to_string(),
                    signature_tech: format!("typealias {name} = {target}"),
                    human_desc: None,
                    doc_comment: doc,
                });
                doc_buffer.clear();
                continue;
            }

            // 6. Funciones
            if let Some(caps) = self.fn_re.captures(trimmed) {
                let fn_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let params = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                let ret = caps.get(3).map(|m| m.as_str().trim()).unwrap_or("");

                let sig_tech = if ret.is_empty() {
                    format!("fun {fn_name}({params})")
                } else {
                    format!("fun {fn_name}({params}): {ret}")
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

impl Default for KotlinParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceParser for KotlinParser {
    fn can_parse(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("kt" | "kts")
        )
    }

    fn parse_file(&self, file_path: &Path, repo_root: &Path) -> Result<Vec<SymbolEntry>> {
        let content = fs::read_to_string(file_path)
            .with_context(|| format!("No se pudo leer archivo Kotlin {}", file_path.display()))?;

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
    fn test_kotlin_parser_extracts_symbols() {
        let parser = KotlinParser::new();
        let code = r#"
package com.licarayen.connect.channel

import io.flutter.embedding.engine.FlutterEngine

/**
 * Canal nativo Android para gestión de notificaciones y Bluetooth.
 */
class AndroidBridgePlugin(private val context: Context) : FlutterPlugin {
    fun configureChannels(engine: FlutterEngine) {}
}

data class DeviceTelemetry(val batteryLevel: Int, val isCharging: Boolean)

interface ChannelCallback {
    fun onEventReceived(payload: String)
}

enum class BridgeState {
    CONNECTED,
    DISCONNECTED
}

object NativeLogger {
    fun log(msg: String) {}
}

/**
 * Consulta la versión nativa del SDK de Android.
 */
suspend fun querySdkVersion(): Int {
    return Build.VERSION.SDK_INT
}

private fun internalSecurityCheck() {}
"#;

        let symbols = parser.parse_content(code, "android/BridgePlugin.kt").unwrap();

        assert_eq!(symbols.len(), 9);
        assert_eq!(symbols[0].name, "AndroidBridgePlugin");
        assert_eq!(symbols[0].kind, SymbolKind::Struct);
        assert_eq!(symbols[0].doc_comment.as_deref(), Some("Canal nativo Android para gestión de notificaciones y Bluetooth."));

        assert_eq!(symbols[1].name, "configureChannels()");
        assert_eq!(symbols[1].kind, SymbolKind::Function);

        assert_eq!(symbols[2].name, "DeviceTelemetry");
        assert_eq!(symbols[2].kind, SymbolKind::Struct);

        assert_eq!(symbols[3].name, "ChannelCallback");
        assert_eq!(symbols[3].kind, SymbolKind::Trait);

        assert_eq!(symbols[4].name, "onEventReceived()");
        assert_eq!(symbols[4].kind, SymbolKind::Function);

        assert_eq!(symbols[5].name, "BridgeState");
        assert_eq!(symbols[5].kind, SymbolKind::Enum);

        assert_eq!(symbols[6].name, "NativeLogger");
        assert_eq!(symbols[6].kind, SymbolKind::Struct);

        assert_eq!(symbols[7].name, "log()");
        assert_eq!(symbols[7].kind, SymbolKind::Function);

        assert_eq!(symbols[8].name, "querySdkVersion()");
        assert_eq!(symbols[8].kind, SymbolKind::Function);
        assert_eq!(symbols[8].doc_comment.as_deref(), Some("Consulta la versión nativa del SDK de Android."));
    }

    #[test]
    fn test_kotlin_parser_advanced_features() {
        let parser = KotlinParser::new();
        let code = r#"
package com.licarayen.connect.telemetry

/**
 * Alias de tipo para despacho reactivo de telemetría.
 */
typealias TelemetryCallback = (String, Map<String, Any>) -> Unit

/**
 * Identificador opaco de sesión nativa.
 */
value class SessionToken(val raw: String)

sealed class OrbitalEvent {
    data class IgnitionStarted(val epoch: Long) : OrbitalEvent()
    object ShutdownCompleted : OrbitalEvent()
}

class TelemetryDispatcher {
    companion object Factory {
        fun createDefault(): TelemetryDispatcher {
            return TelemetryDispatcher()
        }
    }

    fun dispatch(event: OrbitalEvent) {}
}

/**
 * Normaliza y sanea cadenas para transmisión por canal de radio.
 */
fun String.sanitizeRadioPayload(): String {
    return this.trim()
}

infix fun String.matchesNode(nodeId: String): Boolean {
    return this == nodeId
}

operator fun SessionToken.plus(other: SessionToken): SessionToken {
    return SessionToken(this.raw + other.raw)
}
"#;

        let symbols = parser.parse_content(code, "telemetry/Dispatcher.kt").unwrap();

        assert_eq!(symbols.len(), 12);

        // 1. typealias TelemetryCallback
        assert_eq!(symbols[0].name, "TelemetryCallback");
        assert_eq!(symbols[0].kind, SymbolKind::TypeAlias);
        assert_eq!(
            symbols[0].doc_comment.as_deref(),
            Some("Alias de tipo para despacho reactivo de telemetría.")
        );

        // 2. value class SessionToken
        assert_eq!(symbols[1].name, "SessionToken");
        assert_eq!(symbols[1].kind, SymbolKind::Struct);
        assert_eq!(
            symbols[1].doc_comment.as_deref(),
            Some("Identificador opaco de sesión nativa.")
        );

        // 3. sealed class OrbitalEvent
        assert_eq!(symbols[2].name, "OrbitalEvent");
        assert_eq!(symbols[2].kind, SymbolKind::Struct);

        // 4. data class IgnitionStarted : OrbitalEvent
        assert_eq!(symbols[3].name, "IgnitionStarted");
        assert_eq!(symbols[3].kind, SymbolKind::Struct);

        // 5. object ShutdownCompleted
        assert_eq!(symbols[4].name, "ShutdownCompleted");
        assert_eq!(symbols[4].kind, SymbolKind::Struct);

        // 6. class TelemetryDispatcher
        assert_eq!(symbols[5].name, "TelemetryDispatcher");
        assert_eq!(symbols[5].kind, SymbolKind::Struct);

        // 7. companion object Factory
        assert_eq!(symbols[6].name, "Factory");
        assert_eq!(symbols[6].kind, SymbolKind::Struct);

        // 8. fun createDefault()
        assert_eq!(symbols[7].name, "createDefault()");
        assert_eq!(symbols[7].kind, SymbolKind::Function);

        // 9. fun dispatch()
        assert_eq!(symbols[8].name, "dispatch()");
        assert_eq!(symbols[8].kind, SymbolKind::Function);

        // 10. Extension function String.sanitizeRadioPayload()
        assert_eq!(symbols[9].name, "String.sanitizeRadioPayload()");
        assert_eq!(symbols[9].kind, SymbolKind::Function);
        assert_eq!(
            symbols[9].doc_comment.as_deref(),
            Some("Normaliza y sanea cadenas para transmisión por canal de radio.")
        );

        // 11. Infix extension function String.matchesNode()
        assert_eq!(symbols[10].name, "String.matchesNode()");
        assert_eq!(symbols[10].kind, SymbolKind::Function);

        // 12. Operator extension function SessionToken.plus()
        assert_eq!(symbols[11].name, "SessionToken.plus()");
        assert_eq!(symbols[11].kind, SymbolKind::Function);
    }
}
