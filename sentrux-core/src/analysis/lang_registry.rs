//! Language registry — maps file extensions to tree-sitter grammars and queries.
//!
//! Singleton initialized once at startup. Each language registration compiles
//! the tree-sitter query pattern; failures are recorded (not panicked) and
//! surfaced to the UI so users know which languages have degraded parsing.

use std::collections::HashMap;
use tree_sitter::{Language, Query};

/// Interface for detecting the programming language of a file.
/// Enables custom language detection strategies or test mocking.
pub trait LanguageDetector {
    /// Detect language from a file extension (e.g., "rs" → "Rust").
    fn detect_from_ext(&self, ext: &str) -> Option<&str>;
    /// Detect language from a filename (e.g., "Dockerfile" → "Docker").
    fn detect_from_filename(&self, name: &str) -> Option<&str>;
}

/// Configuration for a single language: grammar + compiled query + file extensions.
#[allow(dead_code)] // extensions field used indirectly via by_ext index
pub struct LangConfig {
    /// Language identifier string (e.g. "python", "rust")
    pub name: &'static str,
    /// Compiled tree-sitter grammar for parsing
    pub grammar: Language,
    /// Compiled tree-sitter query for structural extraction
    pub query: Query,
    /// File extensions mapped to this language (without dots)
    pub extensions: &'static [&'static str],
}

/// Central registry mapping language names and file extensions to LangConfig.
pub struct LangRegistry {
    by_name: HashMap<&'static str, usize>,
    by_ext: HashMap<&'static str, usize>,
    configs: Vec<LangConfig>,
    /// Languages that failed to register (query compilation errors).
    /// Exposed so the UI can surface these instead of silently degrading. [ref:4f5a9de5]
    failed_langs: Vec<LangRegistrationError>,
}

/// Structured error for a language that failed to register.
#[derive(Debug, Clone)]
pub struct LangRegistrationError {
    /// Language name that failed to register
    pub lang: &'static str,
    /// File extensions that would have been mapped
    pub extensions: &'static [&'static str],
    /// Query compilation error message
    pub error: String,
}

/// Global singleton — initialized once, immutable after.
static REGISTRY: std::sync::LazyLock<LangRegistry> = std::sync::LazyLock::new(LangRegistry::init);

impl LangRegistry {
    fn init() -> Self {
        let mut registry = LangRegistry {
            by_name: HashMap::new(),
            by_ext: HashMap::new(),
            configs: Vec::new(),
            failed_langs: Vec::new(),
        };

        registry.register_core_languages();
        registry.register_systems_languages();
        registry.register_web_languages();
        registry.register_other_languages();

        registry
    }

    /// Core scripting and compiled languages: Python, JS/TS variants, Rust, Go.
    fn register_core_languages(&mut self) {
        self.register_static(
            "python",
            tree_sitter_python::LANGUAGE.into(),
            include_str!("../queries/python/tags.scm"),
            &["py"],
        );
        self.register_static(
            "javascript",
            tree_sitter_javascript::LANGUAGE.into(),
            include_str!("../queries/javascript/tags.scm"),
            &["js", "mjs", "cjs"],
        );
        self.register_static(
            "jsx",
            tree_sitter_javascript::LANGUAGE.into(),
            include_str!("../queries/javascript/tags.scm"),
            &["jsx"],
        );
        self.register_static(
            "typescript",
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            include_str!("../queries/typescript/tags.scm"),
            &["ts", "mts", "cts"],
        );
        self.register_static(
            "tsx",
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            include_str!("../queries/typescript/tags.scm"),
            &["tsx"],
        );
        self.register_static(
            "rust",
            tree_sitter_rust::LANGUAGE.into(),
            include_str!("../queries/rust/tags.scm"),
            &["rs"],
        );
        self.register_static(
            "go",
            tree_sitter_go::LANGUAGE.into(),
            include_str!("../queries/go/tags.scm"),
            &["go"],
        );
    }

    /// Systems and enterprise languages: C, C++, Java, Ruby, C#, PHP, Bash.
    fn register_systems_languages(&mut self) {
        self.register_static(
            "c",
            tree_sitter_c::LANGUAGE.into(),
            include_str!("../queries/c/tags.scm"),
            &["c"],
        );
        self.register_static(
            "cpp",
            tree_sitter_cpp::LANGUAGE.into(),
            include_str!("../queries/cpp/tags.scm"),
            // .h is commonly C++ headers (classes, templates, namespaces).
            // The C++ grammar is a superset of C, so it handles both correctly.
            &["cpp", "cc", "cxx", "hpp", "h"],
        );
        self.register_static(
            "java",
            tree_sitter_java::LANGUAGE.into(),
            include_str!("../queries/java/tags.scm"),
            &["java"],
        );
        self.register_static(
            "ruby",
            tree_sitter_ruby::LANGUAGE.into(),
            include_str!("../queries/ruby/tags.scm"),
            &["rb"],
        );
        self.register_static(
            "csharp",
            tree_sitter_c_sharp::LANGUAGE.into(),
            include_str!("../queries/csharp/tags.scm"),
            &["cs"],
        );
        self.register_static(
            "php",
            tree_sitter_php::LANGUAGE_PHP.into(),
            include_str!("../queries/php/tags.scm"),
            &["php"],
        );
        self.register_static(
            "bash",
            tree_sitter_bash::LANGUAGE.into(),
            include_str!("../queries/bash/tags.scm"),
            &["sh", "bash"],
        );
    }

    /// Frontend, mobile, and design languages: HTML, CSS, SCSS, Kotlin, Swift, Lua, Scala.
    fn register_web_languages(&mut self) {
        self.register_static(
            "html",
            tree_sitter_html::LANGUAGE.into(),
            include_str!("../queries/html/tags.scm"),
            &["html", "htm"],
        );
        self.register_static(
            "css",
            tree_sitter_css::LANGUAGE.into(),
            include_str!("../queries/css/tags.scm"),
            &["css"],
        );
        // SASS (.sass) uses indentation-based syntax (no braces/semicolons),
        // which is fundamentally different from SCSS. Don't map .sass to SCSS grammar.
        self.register_static(
            "scss",
            tree_sitter_scss::language(),
            include_str!("../queries/scss/tags.scm"),
            &["scss"],
        );
        // kotlin: tree-sitter-kotlin requires tree-sitter 0.20/0.21, incompatible with 0.25
        // Will re-add when tree-sitter-kotlin supports 0.25+
        self.register_static(
            "swift",
            tree_sitter_swift::LANGUAGE.into(),
            include_str!("../queries/swift/tags.scm"),
            &["swift"],
        );
        self.register_static(
            "lua",
            tree_sitter_lua::LANGUAGE.into(),
            include_str!("../queries/lua/tags.scm"),
            &["lua"],
        );
        self.register_static(
            "scala",
            tree_sitter_scala::LANGUAGE.into(),
            include_str!("../queries/scala/tags.scm"),
            &["scala", "sc"],
        );
    }

    /// Additional languages (Phase 5): Elixir, Haskell, Zig, R, Dockerfile, OCaml.
    fn register_other_languages(&mut self) {
        self.register_static(
            "elixir",
            tree_sitter_elixir::LANGUAGE.into(),
            include_str!("../queries/elixir/tags.scm"),
            &["ex", "exs"],
        );
        self.register_static(
            "haskell",
            tree_sitter_haskell::LANGUAGE.into(),
            include_str!("../queries/haskell/tags.scm"),
            &["hs"],
        );
        self.register_static(
            "zig",
            tree_sitter_zig::LANGUAGE.into(),
            include_str!("../queries/zig/tags.scm"),
            &["zig"],
        );
        self.register_static(
            "r",
            tree_sitter_r::LANGUAGE.into(),
            include_str!("../queries/r/tags.scm"),
            &["r", "R"],
        );
        // dockerfile: tree-sitter-dockerfile requires tree-sitter 0.20, incompatible with 0.25
        self.register_static(
            "ocaml",
            tree_sitter_ocaml::LANGUAGE_OCAML.into(),
            include_str!("../queries/ocaml/tags.scm"),
            &["ml"],
        );
        self.register_static(
            "ocaml_interface",
            tree_sitter_ocaml::LANGUAGE_OCAML_INTERFACE.into(),
            include_str!("../queries/ocaml/tags.scm"),
            &["mli"],
        );
    }

    /// Register a language from a tree-sitter 0.20 crate by transmuting to 0.25.
    ///
    /// SAFETY: Both tree_sitter 0.20::Language and 0.25::Language are
    /// #[repr(transparent)] wrappers around *const ffi::TSLanguage (the C struct).
    /// The C ABI (TSLanguage) is stable across these versions. The transmute only
    /// reinterprets the Rust wrapper type, not the underlying pointer. We validate
    /// size, alignment, and grammar ABI version at runtime before using the language.
    fn register_transmuted_lang<L: Copy>(
        &mut self,
        name: &'static str,
        old_lang: L,
        query_src: &str,
        extensions: &'static [&'static str],
    ) {
        assert_eq!(
            std::mem::size_of::<L>(),
            std::mem::size_of::<Language>(),
            "tree-sitter Language type size mismatch for {} — unsafe transmute would be UB",
            name,
        );
        assert_eq!(
            std::mem::align_of::<L>(),
            std::mem::align_of::<Language>(),
            "tree-sitter Language type alignment mismatch for {} — unsafe transmute would be UB",
            name,
        );
        let new_lang: Language = unsafe { std::mem::transmute_copy(&old_lang) };
        // Runtime ABI version check: tree-sitter 0.25 expects version >= 14
        #[allow(deprecated)]
        let version = new_lang.version();
        if version >= 14 {
            self.register_static(name, new_lang, query_src, extensions);
        } else {
            eprintln!(
                "[lang_registry] WARN: {} grammar ABI version {} too old (need >= 14), skipping",
                name, version
            );
        }
    }

    fn register_static(
        &mut self,
        name: &'static str,
        grammar: Language,
        query_src: &str,
        extensions: &'static [&'static str],
    ) {
        match Query::new(&grammar, query_src) {
            Ok(query) => {
                let idx = self.configs.len();
                self.configs.push(LangConfig {
                    name,
                    grammar,
                    query,
                    extensions,
                });
                self.by_name.insert(name, idx);
                for ext in extensions {
                    self.by_ext.insert(ext, idx);
                }
            }
            Err(e) => {
                let err_msg = format!("{:?}", e);
                eprintln!(
                    "[lang_registry] ERROR: query compilation failed for lang='{}', extensions={:?}: {}",
                    name, extensions, err_msg
                );
                self.failed_langs.push(LangRegistrationError {
                    lang: name,
                    extensions,
                    error: err_msg,
                });
            }
        }
    }

    /// Look up by language name (e.g. "python", "rust").
    pub fn get(&self, name: &str) -> Option<&LangConfig> {
        self.by_name.get(name).map(|&idx| &self.configs[idx])
    }

    /// Look up by file extension (without dot, e.g. "py", "rs").
    pub fn get_by_ext(&self, ext: &str) -> Option<&LangConfig> {
        self.by_ext.get(ext).map(|&idx| &self.configs[idx])
    }

    /// All registered file extensions (for graph.rs module-map stripping).
    pub fn all_extensions(&self) -> Vec<&'static str> {
        self.by_ext.keys().copied().collect()
    }

    /// Languages that failed to register due to query compilation errors.
    pub fn failed(&self) -> &[LangRegistrationError] {
        &self.failed_langs
    }
}

// ---- Public free functions delegating to global singleton ----

/// Get language config by name.
pub fn get(name: &str) -> Option<&'static LangConfig> {
    REGISTRY.get(name)
}

/// All registered extensions (e.g. ["py", "rs", "js", ...]).
pub fn all_extensions() -> Vec<&'static str> {
    REGISTRY.all_extensions()
}

/// Detect language name from file extension string.
/// Returns the registry language name if found, otherwise maps common
/// non-parseable extensions to their language names for display.
pub fn detect_lang_from_ext(ext: &str) -> String {
    if let Some(config) = REGISTRY.get_by_ext(ext) {
        return config.name.to_string();
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
        "sass" => "sass".into(), // display-only; SASS syntax differs from SCSS
        _ => "unknown".into(),
    }
}

/// Detect language from the full filename (not just extension).
/// BUG 18 fix: handle common extensionless files beyond just Dockerfile.
/// Makefile/Rakefile/Gemfile are recognized for display (not structural
/// parsing) so they get correct language labels. [ref:93cf32d4]
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

/// Languages that failed query compilation. Non-empty means silent degradation
/// is occurring — the UI should surface these errors. [ref:4f5a9de5]
pub fn failed_languages() -> &'static [LangRegistrationError] {
    REGISTRY.failed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_init_all_queries_compile() {
        // Force initialization — will panic if any query fails to compile
        let _ = &*REGISTRY;
        // Verify all language registrations (jsx/tsx/ocaml_interface are separate)
        for lang in &[
            "python",
            "javascript",
            "jsx",
            "typescript",
            "tsx",
            "rust",
            "go",
            "c",
            "cpp",
            "java",
            "ruby",
            "csharp",
            "php",
            "bash",
            "html",
            "css",
            "scss",
            "swift",
            "lua",
            "scala",
            "elixir",
            "haskell",
            "zig",
            "r",
            "ocaml",
            "ocaml_interface",
        ] {
            assert!(REGISTRY.get(lang).is_some(), "{} missing", lang);
        }
    }

    #[test]
    fn test_ext_lookup() {
        assert_eq!(REGISTRY.get_by_ext("py").unwrap().name, "python");
        assert_eq!(REGISTRY.get_by_ext("rs").unwrap().name, "rust");
        assert_eq!(REGISTRY.get_by_ext("js").unwrap().name, "javascript");
        assert_eq!(REGISTRY.get_by_ext("ts").unwrap().name, "typescript");
        assert_eq!(REGISTRY.get_by_ext("go").unwrap().name, "go");
        assert_eq!(REGISTRY.get_by_ext("tsx").unwrap().name, "tsx");
        assert_eq!(REGISTRY.get_by_ext("jsx").unwrap().name, "jsx");
    }

    #[test]
    fn test_queries_have_expected_captures() {
        for lang_name in &["python", "javascript", "typescript", "rust", "go"] {
            let config = REGISTRY.get(lang_name).unwrap();
            let names: Vec<&str> = config.query.capture_names().to_vec();
            // Accept either official or custom capture conventions
            let has_func = names.contains(&"func.def")
                || names.contains(&"definition.function")
                || names.contains(&"definition.method");
            assert!(has_func, "{} missing function definition capture", lang_name);
            let has_name = names.contains(&"func.name")
                || names.contains(&"name")
                || names.contains(&"call.name");
            assert!(has_name, "{} missing name capture", lang_name);
            let has_call = names.contains(&"call.name")
                || names.contains(&"call")
                || names.contains(&"reference.call");
            assert!(has_call, "{} missing call capture", lang_name);
        }
    }

    #[test]
    fn test_detect_lang_from_ext() {
        assert_eq!(detect_lang_from_ext("py"), "python");
        assert_eq!(detect_lang_from_ext("rs"), "rust");
        assert_eq!(detect_lang_from_ext("json"), "json");
        assert_eq!(detect_lang_from_ext("xyz"), "unknown");
    }
}
