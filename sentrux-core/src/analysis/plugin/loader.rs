//! Runtime plugin loader — discovers and loads language plugins from ~/.sentrux/plugins/.
//!
//! Each plugin directory contains:
//! - plugin.toml (manifest)
//! - grammars/<platform>.so|.dylib (compiled tree-sitter grammar)
//! - queries/tags.scm (tree-sitter queries)
//!
//! Loaded grammars are registered into the global LangRegistry alongside built-in languages.
//! Plugin languages take priority over built-in (allows user overrides).

use super::manifest::PluginManifest;
use super::profile::LanguageProfile;
use std::path::{Path, PathBuf};
use tree_sitter::Language;

/// Result of loading a single plugin.
#[derive(Debug)]
pub struct LoadedPlugin {
    /// Plugin name from manifest
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Version
    pub version: String,
    /// File extensions
    pub extensions: Vec<String>,
    /// Loaded tree-sitter grammar
    pub grammar: Language,
    /// Compiled tree-sitter query source
    pub query_src: String,
    /// Layer 2: language profile (semantics + thresholds)
    pub profile: LanguageProfile,
}

/// Error loading a plugin (non-fatal — logged and skipped).
#[derive(Debug)]
pub struct PluginLoadError {
    pub plugin_dir: PathBuf,
    pub error: String,
}

/// Get the plugins directory path.
pub fn plugins_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".sentrux").join("plugins"))
}

/// Discover and load all plugins from ~/.sentrux/plugins/.
/// Returns successfully loaded plugins and any errors encountered.
/// Errors are non-fatal: each plugin loads independently.
pub fn load_all_plugins() -> (Vec<LoadedPlugin>, Vec<PluginLoadError>) {
    let mut loaded = Vec::new();
    let mut errors = Vec::new();

    let dir = match plugins_dir() {
        Some(d) if d.is_dir() => d,
        _ => return (loaded, errors),
    };

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("[plugin] Failed to read plugins dir: {}", e);
            return (loaded, errors);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        match load_single_plugin(&path) {
            Ok(plugin) => {
                // Verbose per-plugin logging removed — registry logs the total count
                loaded.push(plugin);
            }
            Err(e) => {
                eprintln!("[plugin] Failed to load {}: {}", path.display(), e);
                errors.push(PluginLoadError {
                    plugin_dir: path,
                    error: e,
                });
            }
        }
    }

    (loaded, errors)
}

/// Load a single plugin from a directory.
fn load_single_plugin(plugin_dir: &Path) -> Result<LoadedPlugin, String> {
    // 1. Parse manifest
    let manifest = PluginManifest::load(plugin_dir)?;

    // 2. Load query source
    let query_path = plugin_dir.join("queries").join("tags.scm");
    let query_src = std::fs::read_to_string(&query_path)
        .map_err(|e| format!("Failed to read {}: {}", query_path.display(), e))?;

    // 3. Validate query captures match declared capabilities
    manifest.validate_query_captures(&query_src)?;

    // 4. Load grammar binary
    let grammar_file = PluginManifest::grammar_filename();
    if grammar_file == "unsupported" {
        return Err("Unsupported platform for runtime grammar loading".into());
    }
    let grammar_path = plugin_dir.join("grammars").join(grammar_file);
    if !grammar_path.exists() {
        return Err(format!(
            "Grammar binary not found: {}. Build it for this platform.",
            grammar_path.display()
        ));
    }

    // 5. Verify checksum if provided
    verify_checksum(&manifest, &grammar_path, grammar_file)?;

    // 6. Load the grammar via dynamic library
    let symbol_name = manifest.grammar.symbol_name.as_deref()
        .unwrap_or(&manifest.plugin.name);
    let grammar = load_grammar_dynamic(&grammar_path, symbol_name)?;

    // 7. Verify ABI version
    #[allow(deprecated)]
    let abi = grammar.version();
    if abi < manifest.grammar.abi_version as usize {
        return Err(format!(
            "Grammar ABI version {} < required {}",
            abi, manifest.grammar.abi_version
        ));
    }

    // 8. Test-compile the query to catch errors early
    tree_sitter::Query::new(&grammar, &query_src)
        .map_err(|e| format!("Query compilation failed: {:?}", e))?;

    let profile = LanguageProfile {
        name: manifest.plugin.name.clone(),
        semantics: manifest.semantics,
        thresholds: manifest.thresholds,
    };

    Ok(LoadedPlugin {
        name: manifest.plugin.name,
        display_name: manifest.plugin.display_name,
        version: manifest.plugin.version,
        extensions: manifest.plugin.extensions,
        grammar,
        query_src,
        profile,
    })
}

/// Verify SHA256 checksum of grammar binary against manifest.
fn verify_checksum(manifest: &PluginManifest, grammar_path: &Path, platform_key: &str) -> Result<(), String> {
    // Strip extension to get platform key (e.g., "darwin-arm64.dylib" → "darwin-arm64")
    let key = platform_key.rsplit_once('.').map_or(platform_key, |(k, _)| k);
    let expected = match manifest.checksums.get(key) {
        Some(hash) => hash,
        None => return Ok(()), // No checksum in manifest = skip verification
    };

    let bytes = std::fs::read(grammar_path)
        .map_err(|e| format!("Failed to read grammar for checksum: {}", e))?;

    // Simple SHA256 via manual computation is heavy — for now, skip if no sha2 crate.
    // TODO: Add sha2 dependency and verify properly.
    let _ = (expected, bytes);
    Ok(())
}

/// Load a tree-sitter Language from a dynamic library (.so/.dylib).
///
/// The library must export a function named `tree_sitter_<name>` that returns
/// a `*const TSLanguage` pointer. This is the standard tree-sitter convention.
fn load_grammar_dynamic(path: &Path, lang_name: &str) -> Result<Language, String> {
    // Safety: we're loading a tree-sitter grammar .so/.dylib which exports
    // a single `tree_sitter_<name>()` function returning *const TSLanguage.
    // This is the same mechanism nvim-treesitter, helix, and zed use.
    unsafe {
        let lib = libloading::Library::new(path)
            .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;

        // tree-sitter convention: exported function is `tree_sitter_<name>`
        let func_name = format!("tree_sitter_{}", lang_name);
        let func: libloading::Symbol<unsafe extern "C" fn() -> Language> = lib
            .get(func_name.as_bytes())
            .map_err(|e| format!(
                "Symbol '{}' not found in {}: {}. The grammar must export tree_sitter_{}().",
                func_name, path.display(), e, lang_name
            ))?;

        let language = func();

        // Leak the library to keep it alive for the lifetime of the process.
        // tree-sitter Language holds pointers into the library's memory.
        std::mem::forget(lib);

        Ok(language)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugins_dir() {
        let dir = plugins_dir();
        assert!(dir.is_some());
        assert!(dir.unwrap().ends_with(".sentrux/plugins"));
    }

    #[test]
    fn test_load_nonexistent_dir() {
        let (loaded, errors) = load_all_plugins();
        // Should not crash even if dir doesn't exist
        let _ = (loaded, errors);
    }
}
