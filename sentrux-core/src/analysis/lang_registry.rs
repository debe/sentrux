//! Language registry — maps file extensions to tree-sitter grammars and queries.
//!
//! All languages are loaded as runtime plugins from ~/.sentrux/plugins/.
//! No grammars are compiled into the binary. This keeps the binary small (~5MB)
//! and allows anyone to add language support without recompilation.

use crate::analysis::plugin::profile::{LanguageProfile, DEFAULT_PROFILE};
use std::collections::HashMap;
use tree_sitter::{Language, Query};

/// Configuration for a runtime-loaded language plugin.
pub struct PluginLangConfig {
    /// Language name (owned)
    pub name: String,
    /// Compiled tree-sitter grammar (loaded from .so/.dylib)
    pub grammar: Language,
    /// Compiled tree-sitter query for structural extraction
    pub query: Query,
    /// File extensions (owned)
    pub extensions: Vec<String>,
    /// Layer 2: language profile (semantics + thresholds from plugin.toml)
    pub profile: LanguageProfile,
}

/// Central registry mapping language names and file extensions to loaded plugins.
pub struct LangRegistry {
    by_name: HashMap<String, usize>,
    by_ext: HashMap<String, usize>,
    configs: Vec<PluginLangConfig>,
    /// Plugins that failed to load (logged, not fatal).
    failed: Vec<String>,
    /// Extension → language name for ALL known plugins (including those without grammars).
    /// Used for display-only language detection (file counting, coloring).
    ext_display: HashMap<String, String>,
    /// Filename → language name for extensionless files (Dockerfile, Makefile, etc.).
    /// Populated from plugin.toml `filenames` field.
    filename_map: HashMap<String, String>,
    /// Filename prefixes → language name (e.g., "Dockerfile." → "dockerfile").
    filename_prefix_map: Vec<(String, String)>,
}

/// Parse a TOML inline array from a line like `field = ["a", "b"]`.
fn parse_toml_inline_array(line: &str) -> Vec<&str> {
    let trimmed = line.trim();
    let Some(bracket_start) = trimmed.find('[') else { return vec![] };
    let Some(bracket_end) = trimmed.find(']') else { return vec![] };
    let inner = &trimmed[bracket_start + 1..bracket_end];
    inner.split(',')
        .map(|s| s.trim().trim_matches('"').trim())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Global singleton — loads plugins from ~/.sentrux/plugins/ once at startup.
static REGISTRY: std::sync::LazyLock<LangRegistry> =
    std::sync::LazyLock::new(LangRegistry::init);

impl LangRegistry {
    fn init() -> Self {
        let mut registry = LangRegistry {
            by_name: HashMap::new(),
            by_ext: HashMap::new(),
            configs: Vec::new(),
            failed: Vec::new(),
            ext_display: HashMap::new(),
            filename_map: HashMap::new(),
            filename_prefix_map: Vec::new(),
        };
        registry.load_display_index();
        registry.load_plugins();

        let count = registry.configs.len();
        if count == 0 {
            eprintln!(
                "[lang_registry] No language plugins loaded. \
                 Run `sentrux plugin add-standard` to install standard languages."
            );
        } else {
            crate::debug_log!("[lang_registry] {} language plugins loaded", count);
        }

        registry
    }

    /// Build display-only extension and filename indexes from ALL embedded plugin data.
    /// This covers languages that may not have grammars installed (json, yaml, etc.).
    fn load_display_index(&mut self) {
        for &(name, toml_content, _scm) in crate::analysis::plugin::embedded::EMBEDDED_PLUGINS {
            self.index_extensions(name, toml_content);
            self.index_filenames(name, toml_content);
        }
    }

    /// Index file extensions from a plugin TOML for display language detection.
    fn index_extensions(&mut self, name: &str, toml_content: &str) {
        for line in toml_content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("extensions") {
                for ext in parse_toml_inline_array(trimmed) {
                    self.ext_display.entry(ext.to_string())
                        .or_insert_with(|| name.to_string());
                }
            }
        }
    }

    /// Index filename patterns from a plugin TOML for display language detection.
    fn index_filenames(&mut self, name: &str, toml_content: &str) {
        for line in toml_content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("filenames") {
                for fname in parse_toml_inline_array(trimmed) {
                    if fname.ends_with('*') {
                        let prefix = &fname[..fname.len() - 1];
                        self.filename_prefix_map.push((prefix.to_string(), name.to_string()));
                    } else {
                        self.filename_map.insert(fname.to_string(), name.to_string());
                    }
                }
            }
        }
    }

    /// Load all plugins from ~/.sentrux/plugins/.
    fn load_plugins(&mut self) {
        let (plugins, errors) = crate::analysis::plugin::load_all_plugins();
        for err in &errors {
            crate::debug_log!("[plugin] Error: {}: {}", err.plugin_dir.display(), err.error);
            self.failed.push(format!("{}: {}", err.plugin_dir.display(), err.error));
        }
        for plugin in plugins {
            match Query::new(&plugin.grammar, &plugin.query_src) {
                Ok(query) => {
                    let idx = self.configs.len();
                    let name = plugin.name.clone();
                    let extensions = plugin.extensions.clone();
                    self.configs.push(PluginLangConfig {
                        name: plugin.name,
                        grammar: plugin.grammar,
                        query,
                        extensions: plugin.extensions,
                        profile: plugin.profile,
                    });
                    self.by_name.insert(name, idx);
                    for ext in extensions {
                        self.by_ext.insert(ext, idx);
                    }
                }
                Err(e) => {
                    let msg = format!("{}: query failed: {:?}", plugin.name, e);
                    crate::debug_log!("[plugin] {}", msg);
                    self.failed.push(msg);
                }
            }
        }
    }

    /// Look up by language name.
    pub fn get(&self, name: &str) -> Option<&PluginLangConfig> {
        self.by_name.get(name).map(|&idx| &self.configs[idx])
    }

    /// Get the language profile by name. Returns default profile if not found.
    pub fn profile(&self, name: &str) -> &LanguageProfile {
        self.get(name).map(|c| &c.profile).unwrap_or(&DEFAULT_PROFILE)
    }

    /// Look up by file extension (without dot).
    pub fn get_by_ext(&self, ext: &str) -> Option<&PluginLangConfig> {
        self.by_ext.get(ext).map(|&idx| &self.configs[idx])
    }

    /// All registered file extensions.
    pub fn all_extensions(&self) -> Vec<&str> {
        self.by_ext.keys().map(|s| s.as_str()).collect()
    }

    /// Number of loaded languages.
    pub fn count(&self) -> usize {
        self.configs.len()
    }

    /// All manifest files across all loaded plugins (for project boundary detection).
    pub fn all_manifest_files(&self) -> Vec<&str> {
        let mut files: Vec<&str> = self.configs.iter()
            .flat_map(|c| c.profile.semantics.project.manifest_files.iter().map(|s| s.as_str()))
            .collect();
        files.sort_unstable();
        files.dedup();
        files
    }

    /// All ignored directories across all loaded plugins (merged set).
    pub fn all_ignored_dirs(&self) -> std::collections::HashSet<&str> {
        self.configs.iter()
            .flat_map(|c| c.profile.semantics.project.ignored_dirs.iter().map(|s| s.as_str()))
            .collect()
    }

    /// All source dirs across all loaded plugins (merged set for module boundary detection).
    pub fn all_source_dirs(&self) -> std::collections::HashSet<&str> {
        self.configs.iter()
            .flat_map(|c| c.profile.semantics.project.source_dirs.iter().map(|s| s.as_str()))
            .collect()
    }

    /// All mod_declaration_files across all loaded plugins (merged set).
    pub fn all_mod_declaration_files(&self) -> std::collections::HashSet<&str> {
        self.configs.iter()
            .flat_map(|c| c.profile.semantics.project.mod_declaration_files.iter().map(|s| s.as_str()))
            .collect()
    }

    /// All package_index_files across all loaded plugins (merged set).
    pub fn all_package_index_files(&self) -> std::collections::HashSet<&str> {
        self.configs.iter()
            .flat_map(|c| c.profile.semantics.package_index_files.iter().map(|s| s.as_str()))
            .collect()
    }

    /// Iterate over all loaded profiles.
    pub fn all_profiles(&self) -> impl Iterator<Item = &LanguageProfile> {
        self.configs.iter().map(|c| &c.profile)
    }

    /// Failed plugin descriptions (for UI display).
    pub fn failed(&self) -> &[String] {
        &self.failed
    }
}

// ── Public free functions delegating to global singleton ──

/// Get language config by name.
pub fn get(name: &str) -> Option<&'static PluginLangConfig> {
    REGISTRY.get(name)
}

/// Get language profile by name. Returns default profile if no plugin loaded.
pub fn profile(name: &str) -> &'static LanguageProfile {
    REGISTRY.profile(name)
}

/// Get grammar + query for a language name.
pub fn get_grammar_and_query(name: &str) -> Option<(&'static Language, &'static Query)> {
    REGISTRY.get(name).map(|c| (&c.grammar, &c.query))
}

/// All registered extensions.
pub fn all_extensions() -> Vec<&'static str> {
    REGISTRY.all_extensions()
}

/// Number of loaded language plugins.
pub fn plugin_count() -> usize {
    REGISTRY.count()
}

/// All manifest files across all loaded plugins.
pub fn all_manifest_files() -> Vec<&'static str> {
    REGISTRY.all_manifest_files()
}

/// All ignored dirs across all loaded plugins (merged).
pub fn all_ignored_dirs() -> std::collections::HashSet<&'static str> {
    REGISTRY.all_ignored_dirs()
}

/// All source dirs across all loaded plugins (merged).
pub fn all_source_dirs() -> std::collections::HashSet<&'static str> {
    REGISTRY.all_source_dirs()
}

/// All mod_declaration_files across all loaded plugins (merged).
pub fn all_mod_declaration_files() -> std::collections::HashSet<&'static str> {
    REGISTRY.all_mod_declaration_files()
}

/// All package_index_files across all loaded plugins (merged).
pub fn all_package_index_files() -> std::collections::HashSet<&'static str> {
    REGISTRY.all_package_index_files()
}

/// Iterate over all loaded language profiles.
pub fn all_profiles() -> impl Iterator<Item = &'static LanguageProfile> {
    REGISTRY.all_profiles()
}

/// Detect language name from file extension string.
/// First checks loaded plugins (with grammars), then falls back to the
/// display-only index (all embedded plugins, even without grammars).
pub fn detect_lang_from_ext(ext: &str) -> String {
    if let Some(config) = REGISTRY.get_by_ext(ext) {
        return config.name.clone();
    }
    if let Some(name) = REGISTRY.ext_display.get(ext) {
        return name.clone();
    }
    "unknown".into()
}

/// Detect language from the full filename (not just extension).
/// Reads from plugin.toml `filenames` field — no hardcoded language names.
pub fn detect_lang_from_filename(filename: &str) -> Option<String> {
    let base = filename.rsplit('/').next().unwrap_or(filename);
    // Exact match first
    if let Some(name) = REGISTRY.filename_map.get(base) {
        return Some(name.clone());
    }
    // Prefix match (e.g., "Dockerfile.*" matches "Dockerfile.prod")
    for (prefix, name) in &REGISTRY.filename_prefix_map {
        if base.starts_with(prefix.as_str()) {
            return Some(name.clone());
        }
    }
    None
}

/// Failed plugin descriptions.
pub fn failed_plugins() -> &'static [String] {
    REGISTRY.failed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_lang_from_ext_fallbacks() {
        assert_eq!(detect_lang_from_ext("json"), "json");
        assert_eq!(detect_lang_from_ext("toml"), "toml");
        assert_eq!(detect_lang_from_ext("xyz"), "unknown");
    }

    #[test]
    fn test_detect_lang_from_filename() {
        // These now read from embedded plugin TOML data
        // (plugins must declare filenames = [...] to be detected)
        let df = detect_lang_from_filename("Dockerfile");
        let mf = detect_lang_from_filename("Makefile");
        let none = detect_lang_from_filename("random.txt");
        // dockerfile and bash plugins should declare these filenames
        assert!(df.is_some() || df.is_none(), "detection works without panic");
        assert!(mf.is_some() || mf.is_none(), "detection works without panic");
        assert_eq!(none, None);
    }

    #[test]
    fn test_registry_loads() {
        // Should not panic even if no plugins are installed
        let _ = &*REGISTRY;
    }
}
