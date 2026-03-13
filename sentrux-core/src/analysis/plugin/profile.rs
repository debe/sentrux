//! Language profile — Layer 2 of the plugin architecture.
//!
//! A `LanguageProfile` encapsulates all semantic knowledge and grading thresholds
//! for a single language. It is deserialized from the `[semantics]` and `[thresholds]`
//! sections of plugin.toml and replaces all `match lang { ... }` chains in the codebase.
//!
//! Three-level precedence (lowest to highest):
//!   1. Compiled defaults (`Default` impl) — universal baselines from research
//!   2. Plugin `[thresholds]` / `[semantics]` — language-specific norms
//!   3. Project `.sentrux/rules.toml` — project-level policy overrides
//!
//! Architecture:
//!   Layer 1 (plugin.toml [grammar] + [queries])  → HOW to parse
//!   Layer 2 (plugin.toml [semantics] + [thresholds]) → WHAT'S NORMAL (this module)
//!   Layer 3 (.sentrux/rules.toml)                → WHAT THE PROJECT REQUIRES

use serde::Deserialize;

// ── Semantics: language-specific knowledge ──

/// How this language's import system, comments, and type system work.
/// Deserialized from `[semantics]` in plugin.toml.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LanguageSemantics {
    // ── Import system ──

    /// Whether `.` is a module separator (Python: `os.path` → `os/path`).
    /// If false, `.` is treated as file extension (C: `stdio.h`).
    pub dot_is_module_separator: bool,

    /// Key into the compiled import extractor registry.
    /// Languages with complex import syntax (Rust brace expansion, Python relative
    /// imports, Go go.mod stripping) need compiled extractors. Simple languages
    /// use "fallback".
    pub import_extractor: String,

    /// Key into the compiled base-class extractor registry.
    /// Each language has different AST node kinds for inheritance
    /// (Python: `argument_list`, Java: `superclass`, TS: `class_heritage`).
    pub base_class_extractor: String,

    // ── Comment & string syntax ──

    /// Whether `#` starts a line comment (Python, Ruby, Bash, R).
    pub hash_is_comment: bool,

    /// Whether the language has triple-quoted strings (Python `"""..."""`).
    /// Enables the triple-quote state machine in string stripping.
    pub has_triple_quote_strings: bool,

    // ── Module resolution ──

    /// Filenames that represent "directory as module" / barrel re-exporters.
    /// These files' fan-in reflects re-exports, not genuine coupling.
    /// Examples: `["__init__.py"]` for Python, `["mod.rs"]` for Rust.
    pub package_index_files: Vec<String>,

    // ── Abstract type detection (Martin 2003 Distance from Main Sequence) ──

    /// Base class names that indicate an abstract type.
    /// Examples: Python `["Protocol", "ABC", "ABCMeta"]`.
    /// Used in `is_abstract_kind()` fallback when tree-sitter capture doesn't
    /// distinguish abstract vs concrete (e.g., Python Protocol is tagged as `class`).
    pub abstract_base_classes: Vec<String>,

    /// Keywords in class definition that indicate abstractness.
    /// Examples: Java/C# `["abstract"]`, Kotlin `["abstract", "sealed"]`.
    /// Checked against the source text of the class definition node.
    pub abstract_keywords: Vec<String>,

    // ── Entry point detection ──

    /// Whether this language can have executable entry points.
    /// False for CSS, HTML, Markdown, etc.
    pub is_executable: bool,

    /// Filenames (without directory) that indicate application entry points.
    /// Examples: `["main.py", "app.py", "server.py"]` for Python.
    pub main_filenames: Vec<String>,

    // ── Test file detection ──

    /// Directory prefixes that indicate test directories.
    /// Examples: `["test/", "tests/"]` for Python.
    pub test_dir_prefixes: Vec<String>,

    /// Directory infixes that indicate test directories.
    /// Examples: `["/test/", "/tests/"]`.
    pub test_dir_infixes: Vec<String>,

    /// File suffixes that indicate test files.
    /// Examples: `["_test.py"]` for Python, `["_test.rs"]` for Rust.
    pub test_suffixes: Vec<String>,

    /// File prefixes that indicate test files.
    /// Examples: `["test_"]` for Python.
    pub test_prefixes: Vec<String>,

    // ── Complexity keywords ──

    /// Keywords for cyclomatic, cognitive, and nesting complexity counting.
    #[serde(default)]
    pub complexity: ComplexityKeywords,
}

/// Keywords used by the complexity counting algorithms.
/// The algorithms themselves are compiled; these are the language-specific inputs.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ComplexityKeywords {
    /// Cyclomatic complexity branch keywords.
    /// Each string is a pattern matched against stripped source lines.
    /// Include surrounding spaces/tabs for word boundary detection.
    /// Example (Python): `[" if ", "\tif ", "elif ", "for ", "while ", "except ", " and ", " or "]`
    pub cc: Vec<String>,

    /// Cognitive complexity branch keywords (increment B per occurrence).
    /// Example (Rust): `["if ", "else if", "for ", "while ", "loop ", "match "]`
    pub cog_branch: Vec<String>,

    /// Cognitive complexity nesting keywords (increment nesting depth).
    /// Example (Rust): `["if ", "for ", "while ", "loop ", "match "]`
    pub cog_nesting: Vec<String>,
}

// ── Thresholds: what's normal for this language ──

/// Per-language metric thresholds.
/// Deserialized from `[thresholds]` in plugin.toml.
/// These override the universal defaults (from research) with language-specific norms.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LanguageThresholds {
    /// Cyclomatic complexity threshold for "complex function" flag.
    /// Universal default: 15 (McCabe 1976 + NIST SP 500-235, extended CC).
    /// Rust override: 20 (match arms inflate CC without cognitive load).
    pub cc_high: u32,

    /// Maximum function length before flagged as "long".
    /// Universal default: 50 lines.
    pub func_length: u32,

    /// Cognitive complexity threshold.
    /// Universal default: 15 (SonarSource 2016).
    pub cog_high: u32,

    /// Parameter count threshold.
    /// Universal default: 4 (Code Complete, McConnell 2004).
    pub param_high: u32,

    /// Fan-out threshold for god-file detection.
    /// Universal default: 15.
    pub fan_out: usize,

    /// Fan-in threshold for hotspot detection.
    /// Universal default: 20.
    pub fan_in: usize,

    /// File size threshold (lines) for "large file" flag.
    /// Universal default: 500. Rust: 800 (impl blocks).
    pub large_file_lines: u32,

    /// Minimum comment ratio for healthy documentation.
    /// Universal default: 0.08. Rust: 0.03. Java: 0.12.
    pub comment_ratio_min: f64,
}

// ── Combined profile ──

/// Complete language profile: semantics + thresholds.
/// This is the single object threaded through the analysis pipeline,
/// replacing all `lang: &str` parameters and `match lang` chains.
#[derive(Debug, Clone)]
pub struct LanguageProfile {
    /// Language name (e.g., "rust", "python")
    pub name: String,
    /// Semantic knowledge about this language
    pub semantics: LanguageSemantics,
    /// Metric thresholds for this language
    pub thresholds: LanguageThresholds,
}

// ── Default implementations ──
// These produce the universal baselines used when plugin.toml omits a section.
// Values are chosen to match the current hardcoded behavior exactly,
// ensuring zero behavior change during migration.

impl Default for ComplexityKeywords {
    fn default() -> Self {
        Self {
            // Fallback CC keywords (from imports.rs current fallback match arm)
            cc: vec![
                " if ".into(), "\tif ".into(), "if(".into(),
                "else if".into(), "for ".into(), "for(".into(),
                "while ".into(), "while(".into(), "switch ".into(),
                "case ".into(), "catch ".into(), "&&".into(), "||".into(),
            ],
            // Fallback cognitive branch keywords
            cog_branch: vec![
                "if ".into(), "if(".into(), "else if".into(),
                "for ".into(), "for(".into(), "while ".into(), "while(".into(),
                "switch ".into(), "case ".into(), "catch ".into(),
            ],
            // Fallback cognitive nesting keywords
            cog_nesting: vec![
                "if ".into(), "if(".into(), "for ".into(), "for(".into(),
                "while ".into(), "while(".into(), "switch ".into(),
            ],
        }
    }
}

impl Default for LanguageSemantics {
    fn default() -> Self {
        Self {
            dot_is_module_separator: false,
            import_extractor: "fallback".into(),
            base_class_extractor: "generic".into(),
            hash_is_comment: false,
            has_triple_quote_strings: false,
            package_index_files: Vec::new(),
            abstract_base_classes: Vec::new(),
            abstract_keywords: Vec::new(),
            is_executable: true,
            main_filenames: Vec::new(),
            test_dir_prefixes: Vec::new(),
            test_dir_infixes: Vec::new(),
            test_suffixes: Vec::new(),
            test_prefixes: Vec::new(),
            complexity: ComplexityKeywords::default(),
        }
    }
}

impl Default for LanguageThresholds {
    fn default() -> Self {
        Self {
            // McCabe 1976 + NIST SP 500-235 (extended CC with boolean operators)
            cc_high: 15,
            // Industry consensus
            func_length: 50,
            // SonarSource 2016
            cog_high: 15,
            // Code Complete (McConnell 2004)
            param_high: 4,
            // Sentrux heuristic
            fan_out: 15,
            // Sentrux heuristic
            fan_in: 20,
            // SonarQube convention
            large_file_lines: 500,
            // Accommodates most language idioms
            comment_ratio_min: 0.08,
        }
    }
}

impl Default for LanguageProfile {
    fn default() -> Self {
        Self {
            name: "unknown".into(),
            semantics: LanguageSemantics::default(),
            thresholds: LanguageThresholds::default(),
        }
    }
}

impl LanguageProfile {
    /// Create a profile with given name and all defaults.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Check if a file path is a package index / barrel file for this language.
    pub fn is_package_index_file(&self, path: &str) -> bool {
        if self.semantics.package_index_files.is_empty() {
            return false;
        }
        let filename = path.rsplit('/').next().unwrap_or(path);
        self.semantics.package_index_files.iter().any(|f| f == filename)
    }

    /// Check if a base class name indicates an abstract type for this language.
    pub fn has_abstract_base(&self, bases: Option<&Vec<String>>) -> bool {
        match bases {
            Some(bs) if !self.semantics.abstract_base_classes.is_empty() => {
                bs.iter().any(|b| {
                    let name = b.rsplit('.').next().unwrap_or(b);
                    self.semantics.abstract_base_classes.iter().any(|abc| abc == name)
                })
            }
            _ => false,
        }
    }

    /// Check if a class definition's source text contains an abstract keyword.
    pub fn has_abstract_keyword(&self, source_text: &str) -> bool {
        self.semantics.abstract_keywords.iter().any(|kw| {
            // Match as whole word: "abstract" should match "abstract class"
            // but not "abstractFactory" (check for word boundary after keyword)
            source_text.split_whitespace().any(|word| word == kw.as_str())
        })
    }

    /// Check if a file path matches test file patterns for this language.
    pub fn is_test_file(&self, path: &str) -> bool {
        let sem = &self.semantics;

        // Check directory prefixes
        for prefix in &sem.test_dir_prefixes {
            if path.starts_with(prefix.as_str()) {
                return true;
            }
        }

        // Check directory infixes
        for infix in &sem.test_dir_infixes {
            if path.contains(infix.as_str()) {
                return true;
            }
        }

        // Check file suffixes
        for suffix in &sem.test_suffixes {
            if path.ends_with(suffix.as_str()) {
                return true;
            }
        }

        // Check file prefixes (against filename only)
        if !sem.test_prefixes.is_empty() {
            let filename = path.rsplit('/').next().unwrap_or(path);
            for prefix in &sem.test_prefixes {
                if filename.starts_with(prefix.as_str()) {
                    return true;
                }
            }
        }

        false
    }
}

// ── Global default profile (for unknown / missing plugins) ──

/// Lazily-initialized default profile for languages without a plugin.
/// All consumers fall back to this when `lang_registry::profile(lang)` finds no plugin.
pub static DEFAULT_PROFILE: std::sync::LazyLock<LanguageProfile> =
    std::sync::LazyLock::new(LanguageProfile::default);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_thresholds_match_current_constants() {
        let t = LanguageThresholds::default();
        // These must match the constants in metrics/types.rs exactly.
        // If they diverge, the migration will change behavior.
        assert_eq!(t.cc_high, 15);        // CC_THRESHOLD_HIGH
        assert_eq!(t.func_length, 50);    // FUNC_LENGTH_THRESHOLD
        assert_eq!(t.cog_high, 15);       // COG_THRESHOLD_HIGH
        assert_eq!(t.param_high, 4);      // PARAM_THRESHOLD_HIGH
        assert_eq!(t.fan_out, 15);        // FAN_OUT_THRESHOLD
        assert_eq!(t.fan_in, 20);         // FAN_IN_THRESHOLD
        assert_eq!(t.large_file_lines, 500); // LARGE_FILE_THRESHOLD
    }

    #[test]
    fn package_index_detection() {
        let mut p = LanguageProfile::default();
        p.semantics.package_index_files = vec!["__init__.py".into(), "mod.rs".into()];
        assert!(p.is_package_index_file("src/models/__init__.py"));
        assert!(p.is_package_index_file("src/metrics/mod.rs"));
        assert!(!p.is_package_index_file("src/main.rs"));
        assert!(!p.is_package_index_file("src/models/user.py"));
    }

    #[test]
    fn abstract_base_detection() {
        let mut p = LanguageProfile::default();
        p.semantics.abstract_base_classes = vec!["Protocol".into(), "ABC".into()];
        let bases = vec!["typing.Protocol".into()];
        assert!(p.has_abstract_base(Some(&bases)));
        let bases2 = vec!["SomeClass".into()];
        assert!(!p.has_abstract_base(Some(&bases2)));
        assert!(!p.has_abstract_base(None));
    }

    #[test]
    fn abstract_keyword_detection() {
        let mut p = LanguageProfile::default();
        p.semantics.abstract_keywords = vec!["abstract".into()];
        assert!(p.has_abstract_keyword("public abstract class Foo"));
        assert!(!p.has_abstract_keyword("public class Foo"));
        // Should not match partial words
        assert!(!p.has_abstract_keyword("abstractFactory"));
    }

    #[test]
    fn test_file_detection() {
        let mut p = LanguageProfile::default();
        p.semantics.test_suffixes = vec!["_test.py".into()];
        p.semantics.test_prefixes = vec!["test_".into()];
        p.semantics.test_dir_prefixes = vec!["tests/".into()];
        p.semantics.test_dir_infixes = vec!["/tests/".into()];
        assert!(p.is_test_file("auth_test.py"));
        assert!(p.is_test_file("test_auth.py"));
        assert!(p.is_test_file("tests/test_auth.py"));
        assert!(p.is_test_file("src/tests/conftest.py"));
        assert!(!p.is_test_file("src/auth.py"));
    }

    #[test]
    fn default_semantics_safe() {
        let p = LanguageProfile::default();
        assert!(!p.is_package_index_file("anything.py"));
        assert!(!p.has_abstract_base(Some(&vec!["Protocol".into()])));
        assert!(!p.is_test_file("test_something.py"));
    }

    #[test]
    fn serde_deserialize_partial() {
        // Plugin.toml may have only some fields — serde(default) fills the rest
        let toml_str = r#"
            dot_is_module_separator = true
            hash_is_comment = true
            package_index_files = ["__init__.py"]
        "#;
        let sem: LanguageSemantics = toml::from_str(toml_str).unwrap();
        assert!(sem.dot_is_module_separator);
        assert!(sem.hash_is_comment);
        assert_eq!(sem.package_index_files, vec!["__init__.py"]);
        // Defaults for omitted fields
        assert!(!sem.has_triple_quote_strings);
        assert_eq!(sem.import_extractor, "fallback");
    }

    #[test]
    fn serde_deserialize_thresholds() {
        let toml_str = r#"
            cc_high = 20
            large_file_lines = 800
        "#;
        let t: LanguageThresholds = toml::from_str(toml_str).unwrap();
        assert_eq!(t.cc_high, 20);
        assert_eq!(t.large_file_lines, 800);
        // Defaults for omitted fields
        assert_eq!(t.func_length, 50);
        assert_eq!(t.param_high, 4);
    }
}
