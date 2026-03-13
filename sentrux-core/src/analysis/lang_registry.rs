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
        };
        registry.load_plugins();

        let count = registry.configs.len();
        if count == 0 {
            eprintln!(
                "[lang_registry] No language plugins loaded. \
                 Run `sentrux plugin add-standard` to install standard languages."
            );
        } else {
            eprintln!("[lang_registry] {} language plugins loaded", count);
        }

        registry
    }

    /// Load all plugins from ~/.sentrux/plugins/.
    fn load_plugins(&mut self) {
        let (plugins, errors) = crate::analysis::plugin::load_all_plugins();
        for err in &errors {
            eprintln!("[plugin] Error: {}: {}", err.plugin_dir.display(), err.error);
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
                    eprintln!("[plugin] {}", msg);
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

/// Detect language name from file extension string.
pub fn detect_lang_from_ext(ext: &str) -> String {
    if let Some(config) = REGISTRY.get_by_ext(ext) {
        return config.name.clone();
    }
    // Fallback: languages we recognize for display but don't parse structurally
    match ext {
        "json" => "json".into(),
        "toml" => "toml".into(),
        "yaml" | "yml" => "yaml".into(),
        "md" => "markdown".into(),
        "sql" => "sql".into(),
        "dart" => "dart".into(),
        "xml" => "xml".into(),
        "vue" => "vue".into(),
        "svelte" => "svelte".into(),
        "pl" | "pm" => "perl".into(),
        "sass" => "sass".into(),
        "gd" => "gdscript".into(),
        _ => "unknown".into(),
    }
}

/// Detect language from the full filename (not just extension).
pub fn detect_lang_from_filename(filename: &str) -> Option<&'static str> {
    let base = filename.rsplit('/').next().unwrap_or(filename);
    match base {
        "Dockerfile" => Some("dockerfile"),
        "Makefile" | "GNUmakefile" => Some("bash"),
        "Rakefile" | "Gemfile" | "Guardfile" | "Vagrantfile" => Some("ruby"),
        "Justfile" => Some("bash"),
        _ if base.starts_with("Dockerfile.") => Some("dockerfile"),
        _ if base.starts_with("Makefile.") => Some("bash"),
        _ => None,
    }
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
        assert_eq!(detect_lang_from_filename("Dockerfile"), Some("dockerfile"));
        assert_eq!(detect_lang_from_filename("Makefile"), Some("bash"));
        assert_eq!(detect_lang_from_filename("random.txt"), None);
    }

    #[test]
    fn test_registry_loads() {
        // Should not panic even if no plugins are installed
        let _ = &*REGISTRY;
    }
}
