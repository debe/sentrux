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
    /// Only needed for Python (uses `argument_list` which needs special handling).
    /// Most languages use `base_class_node_kinds` instead (data-driven).
    pub base_class_extractor: String,

    /// AST node kinds that contain base class / parent type declarations.
    /// The platform walks class definition children and collects type identifiers
    /// from nodes matching these kinds. This replaces compiled extractors for
    /// most languages.
    /// Examples: `["class_heritage", "extends_clause"]` for TypeScript,
    ///           `["superclass"]` for Ruby, `["base_clause"]` for C++.
    /// If empty, falls back to `base_class_extractor` key or generic detection.
    #[serde(default)]
    pub base_class_node_kinds: Vec<String>,

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

    // ── Import extraction (AST-based) ──

    /// AST-based import path extraction configuration.
    #[serde(default)]
    pub import_ast: ImportAstConfig,

    // ── Project structure ──

    /// Project structure configuration — manifest files, ignored dirs, source dirs.
    #[serde(default)]
    pub project: ProjectConfig,

    // ── Import resolution ──

    /// Import resolver configuration — path aliases, module prefixes, crate aliases.
    #[serde(default)]
    pub resolver: ResolverConfig,

    // ── Complexity (AST-based) ──

    /// AST node kinds for complexity counting.
    /// The platform walks the tree-sitter AST and counts nodes matching these kinds.
    /// No text scanning — tree-sitter already parsed the structure.
    #[serde(default)]
    pub complexity: ComplexityNodes,

    // Legacy complexity_keywords section is ignored if present in plugin.toml.
    // All complexity analysis uses AST node-based branch_nodes/logic_nodes.
}

/// AST-based import path extraction configuration.
///
/// Tells the generic AST walker HOW to find module paths in tree-sitter import nodes.
/// Two strategies:
///   - `field_read`: read a field/child of the import node (Python, Go, JS, C, Ruby)
///   - `scoped_path`: concatenate scoped identifier chains (Rust, Java)
///
/// When `strategy` is empty, falls back to legacy text-based extractors.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ImportAstConfig {
    /// Extraction strategy: "field_read", "scoped_path", or "" (legacy fallback).
    pub strategy: String,

    // ── field_read strategy ──

    /// Field name on the import node that contains the module path.
    /// Python: "module_name", Go: "path", JS: "source", C: "path".
    pub module_path_field: String,

    /// Node kinds that can appear at the module path position.
    /// Walker tries these in order when the field lookup fails.
    /// Python: ["dotted_name", "relative_import"], Go: ["interpreted_string_literal"].
    pub module_path_node_kinds: Vec<String>,

    /// If the path is inside a string literal, what child node holds the raw content.
    /// Go: "interpreted_string_literal_content", JS: "string_fragment", C: "string_content".
    /// Empty = read node text directly (no string unwrapping).
    pub string_content_kind: String,

    /// If the import node is a container (Go import_declaration), what child kind
    /// holds individual import specs. The walker iterates these children.
    /// Go: "import_spec". Empty = import node itself contains the path.
    pub child_import_kind: String,

    // ── scoped_path strategy ──

    /// Separator for joining scoped path segments. Rust: "::", Java: ".".
    pub path_separator: String,

    /// Node kind for branching use lists. Rust: "use_list".
    pub use_list_kind: String,

    /// Node kinds for scoped path nodes. Rust: ["scoped_identifier", "scoped_use_list"].
    pub scoped_path_kinds: Vec<String>,

    // ── Python relative imports ──

    /// Node kind that indicates a relative import. Python: "relative_import".
    pub relative_import_kind: String,

    /// Child node kind that holds the dots. Python: "import_prefix".
    pub import_prefix_kind: String,

    // ── Post-processing ──

    /// Transform applied to extracted module names.
    /// "pascal_to_snake" for Elixir. Empty = no transform.
    pub module_name_transform: String,

    /// Whether to filter out system includes. C/C++: true.
    pub filter_system_includes: bool,

    /// Node kind for system includes to filter. C: "system_lib_string".
    pub system_include_kind: String,
}

impl Default for ImportAstConfig {
    fn default() -> Self {
        Self {
            strategy: String::new(),
            module_path_field: String::new(),
            module_path_node_kinds: Vec::new(),
            string_content_kind: String::new(),
            child_import_kind: String::new(),
            path_separator: String::new(),
            use_list_kind: String::new(),
            scoped_path_kinds: Vec::new(),
            relative_import_kind: String::new(),
            import_prefix_kind: String::new(),
            module_name_transform: String::new(),
            filter_system_includes: false,
            system_include_kind: String::new(),
        }
    }
}

impl ImportAstConfig {
    /// Whether this profile has AST-based import extraction configured.
    pub fn is_configured(&self) -> bool {
        !self.strategy.is_empty()
    }
}

/// Project structure configuration — how this language's ecosystem organizes projects.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ProjectConfig {
    /// Project manifest files for boundary detection in monorepos.
    /// Examples: `["Cargo.toml"]` for Rust, `["package.json"]` for JS/TS.
    /// Multiple allowed: Python has both `pyproject.toml` and `setup.py`.
    pub manifest_files: Vec<String>,

    /// Directories to ignore during scanning (language build artifacts).
    /// Merged across all loaded plugins at runtime.
    /// Examples: `["target", ".cargo"]` for Rust, `["node_modules"]` for JS.
    pub ignored_dirs: Vec<String>,

    /// Source root directories for module boundary detection.
    /// Files under these dirs get per-file module granularity.
    /// Examples: `["src"]` for Rust, `["cmd", "pkg", "internal"]` for Go.
    pub source_dirs: Vec<String>,

    /// Files that represent module declarations (structural containment).
    /// Import edges FROM these to children are excluded from dependency metrics.
    /// Examples: `["mod.rs", "lib.rs", "main.rs"]` for Rust, `["__init__.py"]` for Python.
    pub mod_declaration_files: Vec<String>,

    /// Whether directories act as implicit packages (no index file needed).
    /// True for Go (any .go file in a dir is part of the package).
    pub directory_is_package: bool,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            manifest_files: Vec::new(),
            ignored_dirs: Vec::new(),
            source_dirs: Vec::new(),
            mod_declaration_files: Vec::new(),
            directory_is_package: false,
        }
    }
}

/// Import resolver configuration — how to resolve import specifiers to files.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ResolverConfig {
    /// File containing module/package prefix for stripping from import specifiers.
    /// Go: `"go.mod"`. Platform reads this file and strips the module path prefix.
    pub module_prefix_file: String,

    /// Directive keyword in module_prefix_file that holds the module path.
    /// Go: `"module"` (parses `module github.com/user/repo`).
    pub module_prefix_directive: String,

    /// Manifest file containing project-name alias for suffix-index.
    /// Rust: `"Cargo.toml"`.
    pub alias_file: String,

    /// Dotted field path in alias_file for the project name.
    /// Rust: `"package.name"`.
    pub alias_field: String,

    /// Transform applied to alias name.
    /// `"hyphen_to_underscore"` for Rust crate names (my-crate → my_crate).
    pub alias_transform: String,

    /// Entry point file that the alias resolves to (relative to project root).
    /// Rust: `"src/lib.rs"`.
    pub alias_entry_point: String,

    /// File containing path alias mappings (replaces oxc_resolver).
    /// JS/TS: `"tsconfig.json"`.
    pub path_alias_file: String,

    /// JSON field path to the alias map.
    /// JS/TS: `"compilerOptions.paths"`.
    pub path_alias_field: String,

    /// JSON field for the base URL.
    /// JS/TS: `"compilerOptions.baseUrl"`.
    pub path_alias_base_url: String,

    /// Extensions to try when resolving imports (in order).
    /// JS/TS: `[".ts", ".tsx", ".js", ".jsx", ".mjs", ".mts"]`.
    /// Empty = use the language's registered extensions.
    pub resolve_extensions: Vec<String>,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            module_prefix_file: String::new(),
            module_prefix_directive: String::new(),
            alias_file: String::new(),
            alias_field: String::new(),
            alias_transform: String::new(),
            alias_entry_point: String::new(),
            path_alias_file: String::new(),
            path_alias_field: String::new(),
            path_alias_base_url: String::new(),
            resolve_extensions: Vec::new(),
        }
    }
}

/// AST node kinds for complexity counting via tree-sitter AST walk.
///
/// The platform walks the function's AST subtree and for each node:
///   - If node.kind() is in `branch_nodes`: CC += 1, COG += 1 + nesting_depth
///   - If node.kind() is in `logic_nodes`: CC += 1, COG += 1
///   - If node.kind() is in `nesting_nodes`: nesting_depth += 1 for children
///
/// This replaces text-based keyword scanning with exact AST analysis.
/// No comment/string stripping needed — tree-sitter handles that.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ComplexityNodes {
    /// Node kinds that count as branch points for cyclomatic complexity.
    /// Each occurrence adds +1 to CC. Also adds (1 + nesting) to cognitive complexity.
    /// Examples: `["if_statement", "for_statement", "while_statement", "except_clause"]`
    pub branch_nodes: Vec<String>,

    /// Node kinds that count as boolean/logic operators.
    /// Each occurrence adds +1 to both CC and cognitive complexity (no nesting penalty).
    /// Examples: `["boolean_operator"]` for Python, `["binary_expression"]` for others
    /// Note: for languages using `binary_expression` for ALL operators, the platform
    /// filters to only count `&&`, `||`, `and`, `or` by checking the operator field.
    pub logic_nodes: Vec<String>,

    /// Operator strings that count as logic operators within logic_nodes.
    /// Used to filter `binary_expression` nodes to only count boolean operators.
    /// Examples: `["&&", "||"]` for C-style, `["and", "or"]` for Python.
    /// If empty, all logic_nodes are counted (e.g., Python's `boolean_operator`
    /// is already specific enough).
    #[serde(default)]
    pub logic_operators: Vec<String>,

    /// Node kinds that increase nesting depth for cognitive complexity.
    /// Children of these nodes get +1 nesting penalty on their branch score.
    /// Usually a subset of branch_nodes (e.g., `if` increases nesting but `elif` does not).
    pub nesting_nodes: Vec<String>,
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
    /// UI color [R, G, B] for this language in the renderer.
    /// Default: [80, 85, 90] (neutral gray).
    pub color_rgb: [u8; 3],
}

// ── Default implementations ──
// These produce the universal baselines used when plugin.toml omits a section.
// Values are chosen to match the current hardcoded behavior exactly,
// ensuring zero behavior change during migration.

impl Default for ComplexityNodes {
    fn default() -> Self {
        Self {
            // Empty = no AST-based complexity counting.
            // The platform falls back to legacy text-based keywords if these are empty.
            branch_nodes: Vec::new(),
            logic_nodes: Vec::new(),
            logic_operators: Vec::new(),
            nesting_nodes: Vec::new(),
        }
    }
}

impl ComplexityNodes {
    /// Whether this profile has AST-based complexity nodes configured.
    /// If false, the platform should fall back to legacy text-based counting.
    pub fn is_configured(&self) -> bool {
        !self.branch_nodes.is_empty()
    }
}

impl Default for LanguageSemantics {
    fn default() -> Self {
        Self {
            dot_is_module_separator: false,
            import_extractor: String::new(),
            base_class_extractor: "generic".into(),
            base_class_node_kinds: Vec::new(),
            import_ast: ImportAstConfig::default(),
            project: ProjectConfig::default(),
            resolver: ResolverConfig::default(),
            hash_is_comment: false,
            has_triple_quote_strings: false,
            package_index_files: Vec::new(),
            abstract_base_classes: Vec::new(),
            abstract_keywords: Vec::new(),
            is_executable: false, // Must be explicitly set by plugins that can have entry points
            main_filenames: Vec::new(),
            test_dir_prefixes: Vec::new(),
            test_dir_infixes: Vec::new(),
            test_suffixes: Vec::new(),
            test_prefixes: Vec::new(),
            complexity: ComplexityNodes::default(),
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
            color_rgb: [80, 85, 90], // Neutral gray for unknown languages
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
        assert_eq!(sem.import_extractor, "");
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
