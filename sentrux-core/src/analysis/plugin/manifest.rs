//! Plugin manifest (plugin.toml) — the single source of truth for a language plugin.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use super::profile::{LanguageSemantics, LanguageThresholds};

/// Root manifest structure parsed from plugin.toml.
#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginInfo,
    pub grammar: GrammarInfo,
    pub queries: QueryInfo,
    #[serde(default)]
    pub checksums: HashMap<String, String>,
    /// Layer 2: language-specific semantic knowledge (optional, defaults apply).
    #[serde(default)]
    pub semantics: LanguageSemantics,
    /// Layer 2: language-specific metric thresholds (optional, defaults apply).
    #[serde(default)]
    pub thresholds: LanguageThresholds,
}

#[derive(Debug, Deserialize)]
pub struct PluginInfo {
    /// Machine-readable name (lowercase, no spaces)
    pub name: String,
    /// Human-readable display name
    pub display_name: String,
    /// Semver version
    pub version: String,
    /// File extensions this plugin handles (without dots)
    pub extensions: Vec<String>,
    /// Minimum sentrux version
    #[serde(default)]
    pub min_sentrux_version: Option<String>,
    /// Optional metadata
    #[serde(default)]
    pub metadata: Option<PluginMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct PluginMetadata {
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GrammarInfo {
    /// Source repo URL
    pub source: String,
    /// Exported symbol name override (default: tree_sitter_<plugin_name>)
    /// Use when the grammar exports a different name (e.g., "php_only" for tree_sitter_php_only)
    pub symbol_name: Option<String>,
    /// Git ref used to build
    #[serde(rename = "ref")]
    pub git_ref: String,
    /// tree-sitter ABI version
    pub abi_version: u32,
}

#[derive(Debug, Deserialize)]
pub struct QueryInfo {
    /// Structural elements this plugin extracts
    pub capabilities: Vec<String>,
}

impl PluginManifest {
    /// Load and parse a plugin.toml from a directory.
    pub fn load(plugin_dir: &Path) -> Result<Self, String> {
        let path = plugin_dir.join("plugin.toml");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
    }

    /// Get the expected grammar filename for the current platform.
    pub fn grammar_filename() -> &'static str {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        { "darwin-arm64.dylib" }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        { "darwin-x86_64.dylib" }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        { "linux-x86_64.so" }
        #[cfg(not(any(
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "x86_64"),
        )))]
        { "unsupported" }
    }

    /// Validate that required capabilities have matching captures in query source.
    /// Accepts multiple naming conventions (func.def, definition.function, func.name, etc.)
    pub fn validate_query_captures(&self, query_src: &str) -> Result<(), String> {
        for cap in &self.queries.capabilities {
            let patterns: &[&str] = match cap.as_str() {
                "functions" => &["func.def", "func.name", "definition.function", "definition.method", "function_definition", "name"],
                "classes" => &["class.def", "class.name", "definition.class", "class_definition"],
                "imports" => &["import.path", "import.name", "import", "source"],
                "calls" => &["call.name", "call", "reference.call"],
                _ => continue,
            };
            let found = patterns.iter().any(|p| query_src.contains(p));
            if !found {
                return Err(format!(
                    "Query missing capture for '{}' capability (expected one of: {})",
                    cap,
                    patterns.iter().map(|p| format!("@{p}")).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        Ok(())
    }
}
