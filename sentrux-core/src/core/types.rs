//! Core data types shared across the entire application.
//!
//! These structs form the canonical representation of scanned source files.
//! All layers (analysis, layout, renderer, metrics) depend on these types.

use serde::{Deserialize, Serialize};

/// A node in the scanned file tree — either a file or a directory.
/// Directories have `children`; files have line/function counts.
/// Serialized to JSON for IPC with the Tauri frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    /// Relative path from scan root (e.g. "src/layout/types.rs")
    pub path: String,
    /// File or directory name (last path component)
    pub name: String,
    /// True if this node represents a directory
    pub is_dir: bool,
    /// Total line count (code + comments + blanks)
    pub lines: u32,
    /// Lines of executable logic (excludes comments and blanks)
    pub logic: u32,
    /// Comment line count
    pub comments: u32,
    /// Blank line count
    pub blanks: u32,
    /// Number of functions/methods detected by the parser
    pub funcs: u32,
    /// Last modification time as Unix epoch seconds
    pub mtime: f64,
    /// Git status code: "A" (added), "M" (modified), "D" (deleted), etc.
    pub gs: String,
    /// Detected programming language (e.g. "rust", "typescript")
    pub lang: String,
    /// Structural analysis (functions, classes, imports) if file was parsed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sa: Option<StructuralAnalysis>,
    /// Child nodes (only present for directories)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileNode>>,
}

/// Structural analysis results for a single file.
/// Populated by the tree-sitter parser when the file is small enough to parse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralAnalysis {
    /// Detected functions with line ranges and complexity
    #[serde(rename = "fn", skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<FuncInfo>>,
    /// Detected classes, interfaces, and type definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cls: Option<Vec<ClassInfo>>,
    /// Import/require targets extracted from source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imp: Option<Vec<String>>,
    /// Call-site identifiers detected in the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub co: Option<Vec<String>>,
    /// Semantic tags for classification (e.g. "test", "config", "entry")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Comment line count from tree-sitter AST. Not serialized — internal use only.
    /// Computed during parse to replace tokei dependency.
    #[serde(skip)]
    pub comment_lines: Option<u32>,
}

/// Information about a single function or method.
/// Field names are abbreviated for compact JSON serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuncInfo {
    /// Function name
    pub n: String,
    /// Start line (1-based)
    pub sl: u32,
    /// End line (1-based)
    pub el: u32,
    /// Line count (el - sl + 1)
    pub ln: u32,
    /// Cyclomatic complexity (extended: includes boolean operators)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<u32>,
    /// Cognitive complexity (SonarSource 2016): nesting-weighted branch count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cog: Option<u32>,
    /// Parameter count (excluding self/this).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pc: Option<u32>,
    /// Body hash for duplication detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bh: Option<u64>,
    /// Dependencies: identifiers this function references
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d: Option<Vec<String>>,
    /// Calls made from within this function (deduped per function).
    /// Populated by parser via line-range containment check.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub co: Option<Vec<String>>,
    /// Whether this function is publicly visible (pub/export/public).
    /// Used by dead code detection: public functions are NOT dead code.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_public: bool,
}

/// Information about a class, interface, or type definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    /// Class/interface name
    pub n: String,
    /// Method names defined in this class
    #[serde(skip_serializing_if = "Option::is_none")]
    pub m: Option<Vec<String>>,
    /// Base classes / parent types (for inheritance graph)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b: Option<Vec<String>>,
    /// Kind: "class", "interface", or "type". Used for abstractness computation
    /// (Martin 2003: Distance from Main Sequence).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub k: Option<String>,
}

/// Cached file info for O(1) lookup by path.
/// Used by renderer (color/stats display) and UI (status bar).
/// Lives in core::types (not app::state) so renderer/UI can access
/// without importing the app layer.
pub struct FileIndexEntry {
    /// Total line count
    pub lines: u32,
    /// Lines of executable logic
    pub logic: u32,
    /// Function/method count
    pub funcs: u32,
    /// Detected programming language
    pub lang: String,
    /// Git status code
    pub gs: String,
    /// Last modification timestamp (Unix epoch seconds)
    pub mtime: f64,
    /// Pre-formatted compact stats line for in-block rendering
    pub stats_line: String,
}

// ---------------------------------------------------------------------------
// Graph edge and vertex types (merged from graph_types.rs)
// ---------------------------------------------------------------------------

/// Common interface for all dependency graph edges.
/// Used by generic graph algorithms (fan-in/out, blast radius, cycles)
/// across import, call, and inherit graphs.
#[allow(dead_code)] // API extension point; implementations exist in arch module
pub trait GraphEdge {
    /// Returns the file path where this edge originates.
    fn source_file(&self) -> &str;
    /// Returns the file path that this edge points to.
    fn target_file(&self) -> &str;
}

/// A function-to-function call edge between two files.
/// Produced by the tree-sitter parser's call-site analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    /// File containing the call site
    pub from_file: String,
    /// Function making the call
    pub from_func: String,
    /// File containing the called function
    pub to_file: String,
    /// Function being called
    pub to_func: String,
}

/// A file-to-file import/require edge.
/// The primary graph used for coupling, levelization, and cycle detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportEdge {
    /// File that contains the import statement
    pub from_file: String,
    /// File being imported
    pub to_file: String,
}

/// An inheritance/implementation edge between two classes across files.
/// Used for the inherit dependency layer and abstractness computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritEdge {
    /// File containing the derived class
    pub child_file: String,
    /// Name of the derived class
    pub child_class: String,
    /// File containing the base class
    pub parent_file: String,
    /// Name of the base class
    pub parent_class: String,
}

impl GraphEdge for CallEdge {
    fn source_file(&self) -> &str { &self.from_file }
    fn target_file(&self) -> &str { &self.to_file }
}

impl GraphEdge for ImportEdge {
    fn source_file(&self) -> &str { &self.from_file }
    fn target_file(&self) -> &str { &self.to_file }
}

impl GraphEdge for InheritEdge {
    fn source_file(&self) -> &str { &self.child_file }
    fn target_file(&self) -> &str { &self.parent_file }
}

/// A detected application entry point (main function, HTTP handler, etc.).
/// Used for execution depth computation and attack surface analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPoint {
    /// File containing the entry point
    pub file: String,
    /// Function name (e.g. "main", "handler")
    pub func: String,
    /// Language of the file
    pub lang: String,
    /// Detection confidence: "high" or "low"
    pub confidence: String,
}

// ═══════════════════════════════════════════════════════════════
// Application error types (merged from error.rs)
// ═══════════════════════════════════════════════════════════════

/// Top-level error type for scan and I/O operations.
/// Serializable so it can be returned through Tauri commands.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Filesystem I/O error (read, write, permission denied)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Invalid or inaccessible directory path
    #[error("Path error: {0}")]
    Path(String),
    /// Scanner-internal error (parse failure, OOM, etc.)
    #[allow(dead_code)]
    #[error("Scan error: {0}")]
    Scan(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
