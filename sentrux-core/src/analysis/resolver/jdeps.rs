//! jdeps integration — resolve Java bytecode dependencies via `jdeps`.
//!
//! Runs `jdeps --dot-output` on compiled class directories or JARs,
//! parses the resulting .dot files, and maps package-level dependencies
//! back to source file paths using the scanned FileNode tree.
//!
//! This complements tree-sitter AST import parsing by catching dependencies
//! invisible at the source level: reflection, annotation processors,
//! generated code, and transitive runtime deps.

use crate::core::types::{FileNode, ImportEdge};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result of a jdeps analysis run.
pub struct JdepsResult {
    /// Additional import edges discovered by jdeps (not already in AST graph)
    pub edges: Vec<ImportEdge>,
    /// Number of jdeps package-level deps that could NOT be mapped to source files
    pub unmapped_count: usize,
}

/// Run jdeps on a Java project and return additional import edges.
///
/// `scan_root`: project root (used to locate build output)
/// `files`: scanned file nodes (used to map FQN → source path)
/// `class_dirs`: explicit class directories/JARs to analyze.
///   If empty, auto-detects common build output dirs.
pub fn resolve_jdeps_edges(
    scan_root: &Path,
    files: &[&FileNode],
    class_dirs: &[PathBuf],
) -> Option<JdepsResult> {
    // Build FQN→source-path index from scanned Java files
    let fqn_index = build_fqn_index(scan_root, files);
    if fqn_index.is_empty() {
        return None; // No Java source files found
    }

    // Find class directories to analyze
    let targets = if class_dirs.is_empty() {
        auto_detect_class_dirs(scan_root)
    } else {
        class_dirs.to_vec()
    };

    if targets.is_empty() {
        return None; // No compiled output found
    }

    // Run jdeps and parse output
    let dot_deps = run_jdeps(&targets)?;
    let (edges, unmapped_count) = map_deps_to_edges(&dot_deps, &fqn_index);

    Some(JdepsResult { edges, unmapped_count })
}

/// Build a map from Java fully-qualified class name to source file path.
/// Infers FQN from the file's path relative to common source roots.
fn build_fqn_index(_scan_root: &Path, files: &[&FileNode]) -> HashMap<String, String> {
    let mut index: HashMap<String, String> = HashMap::new();
    let source_roots = ["src/main/java/", "src/test/java/", "src/", "java/"];

    for file in files {
        if file.lang != "java" {
            continue;
        }
        let rel_path = &file.path;

        // Strip source root prefix to get package path
        let pkg_path = source_roots
            .iter()
            .find_map(|root| rel_path.strip_prefix(root))
            .unwrap_or(rel_path);

        // Convert path to FQN: "com/example/Foo.java" → "com.example.Foo"
        if let Some(stem) = pkg_path.strip_suffix(".java") {
            let fqn = stem.replace('/', ".");
            index.insert(fqn.clone(), rel_path.clone());

            // Also index the package name for package-level deps
            if let Some(dot_pos) = fqn.rfind('.') {
                let pkg = &fqn[..dot_pos];
                // Package → first file in that package (for package-level edges)
                index.entry(pkg.to_string()).or_insert_with(|| rel_path.clone());
            }
        }
    }
    index
}

/// Auto-detect common Java build output directories.
fn auto_detect_class_dirs(scan_root: &Path) -> Vec<PathBuf> {
    let candidates = [
        "target/classes",                    // Maven
        "build/classes/java/main",           // Gradle
        "build/classes",                     // Gradle (older)
        "out/production/classes",            // IntelliJ
        "bin",                               // Eclipse
    ];

    candidates
        .iter()
        .map(|c| scan_root.join(c))
        .filter(|p| p.is_dir())
        .collect()
}

/// Run `jdeps` with dot output and parse the result.
/// Returns a list of (from_package, to_package) tuples.
fn run_jdeps(targets: &[PathBuf]) -> Option<Vec<(String, String)>> {
    let mut cmd = Command::new("jdeps");
    cmd.arg("-verbose:class");
    cmd.arg("-filter:none");

    // Use dot output format for structured parsing
    let tmp_dir = std::env::temp_dir().join("sentrux-jdeps");
    let _ = std::fs::create_dir_all(&tmp_dir);
    cmd.arg("--dot-output");
    cmd.arg(&tmp_dir);

    for target in targets {
        cmd.arg(target);
    }

    let output = cmd.output().ok()?;
    if !output.status.success() {
        crate::debug_log!(
            "[jdeps] command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        // Fall back to parsing verbose text output
        return parse_jdeps_verbose(targets);
    }

    // Parse all .dot files in the temp directory
    let mut deps = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "dot") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    parse_dot_deps(&content, &mut deps);
                }
            }
        }
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&tmp_dir);

    if deps.is_empty() { None } else { Some(deps) }
}

/// Parse jdeps .dot output format.
/// Lines look like: `"com.example.Foo" -> "com.example.Bar";`
fn parse_dot_deps(dot_content: &str, deps: &mut Vec<(String, String)>) {
    for line in dot_content.lines() {
        let line = line.trim();
        if !line.contains("->") {
            continue;
        }
        // Parse: "pkg.Class" -> "pkg.OtherClass"
        let parts: Vec<&str> = line.split("->").collect();
        if parts.len() != 2 {
            continue;
        }
        let from = extract_quoted(parts[0]);
        let to = extract_quoted(parts[1]);
        if let (Some(from), Some(to)) = (from, to) {
            // Skip JDK/JRE internal deps
            if to.starts_with("java.") || to.starts_with("javax.") || to.starts_with("jdk.") {
                continue;
            }
            deps.push((from.to_string(), to.to_string()));
        }
    }
}

/// Extract content between double quotes.
fn extract_quoted(s: &str) -> Option<&str> {
    let start = s.find('"')? + 1;
    let end = s[start..].find('"')? + start;
    Some(&s[start..end])
}

/// Fallback: parse `jdeps -verbose:class` text output directly.
/// Lines look like: `   com.example.Foo  -> com.example.Bar  classes`
fn parse_jdeps_verbose(targets: &[PathBuf]) -> Option<Vec<(String, String)>> {
    let mut cmd = Command::new("jdeps");
    cmd.arg("-verbose:class");
    cmd.arg("-filter:none");
    for target in targets {
        cmd.arg(target);
    }

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut deps = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if !line.contains("->") {
            continue;
        }
        let parts: Vec<&str> = line.split("->").collect();
        if parts.len() != 2 {
            continue;
        }
        let from = parts[0].trim().to_string();
        let to = parts[1].trim().split_whitespace().next().unwrap_or("").to_string();

        if to.is_empty() || to.starts_with("java.") || to.starts_with("javax.") || to.starts_with("jdk.") {
            continue;
        }
        deps.push((from, to));
    }

    if deps.is_empty() { None } else { Some(deps) }
}

/// Map jdeps class-level dependencies to file-level ImportEdge using the FQN index.
fn map_deps_to_edges(
    deps: &[(String, String)],
    fqn_index: &HashMap<String, String>,
) -> (Vec<ImportEdge>, usize) {
    let mut edges = Vec::new();
    let mut unmapped = 0;
    let mut seen = std::collections::HashSet::new();

    for (from_fqn, to_fqn) in deps {
        let from_file = resolve_fqn(from_fqn, fqn_index);
        let to_file = resolve_fqn(to_fqn, fqn_index);

        match (from_file, to_file) {
            (Some(from), Some(to)) if from != to => {
                let key = (from.clone(), to.clone());
                if seen.insert(key) {
                    edges.push(ImportEdge {
                        from_file: from,
                        to_file: to,
                    });
                }
            }
            _ => {
                unmapped += 1;
            }
        }
    }

    (edges, unmapped)
}

/// Resolve a fully-qualified class name to a source file path.
/// Tries exact match first, then package-level match.
fn resolve_fqn<'a>(fqn: &str, index: &'a HashMap<String, String>) -> Option<String> {
    // Exact class match
    if let Some(path) = index.get(fqn) {
        return Some(path.clone());
    }
    // Inner class: com.example.Foo$Bar → com.example.Foo
    if let Some(dollar_pos) = fqn.find('$') {
        let outer = &fqn[..dollar_pos];
        if let Some(path) = index.get(outer) {
            return Some(path.clone());
        }
    }
    // Package-level fallback
    if let Some(dot_pos) = fqn.rfind('.') {
        let pkg = &fqn[..dot_pos];
        if let Some(path) = index.get(pkg) {
            return Some(path.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dot_deps_basic() {
        let dot = r#"
digraph "example.jar" {
    "com.example.Foo" -> "com.example.Bar";
    "com.example.Foo" -> "java.lang.String";
    "com.example.Bar" -> "com.example.Baz";
}
"#;
        let mut deps = Vec::new();
        parse_dot_deps(dot, &mut deps);
        assert_eq!(deps.len(), 2); // java.lang.String filtered out
        assert_eq!(deps[0], ("com.example.Foo".to_string(), "com.example.Bar".to_string()));
        assert_eq!(deps[1], ("com.example.Bar".to_string(), "com.example.Baz".to_string()));
    }

    #[test]
    fn extract_quoted_basic() {
        assert_eq!(extract_quoted(r#"   "hello.world"   "#), Some("hello.world"));
        assert_eq!(extract_quoted("no quotes"), None);
    }

    #[test]
    fn build_fqn_index_maps_correctly() {
        let file = FileNode {
            path: "src/main/java/com/example/Foo.java".to_string(),
            name: "Foo.java".to_string(),
            is_dir: false,
            lines: 100,
            logic: 80,
            comments: 10,
            blanks: 10,
            funcs: 5,
            mtime: 0.0,
            gs: String::new(),
            lang: "java".to_string(),
            sa: None,
            children: None,
        };
        let files = vec![&file];
        let root = Path::new("/project");
        let index = build_fqn_index(root, &files);
        assert!(index.contains_key("com.example.Foo"));
        assert!(index.contains_key("com.example"));
    }

    #[test]
    fn resolve_inner_class() {
        let mut index = HashMap::new();
        index.insert("com.example.Foo".to_string(), "src/main/java/com/example/Foo.java".to_string());
        assert_eq!(
            resolve_fqn("com.example.Foo$Inner", &index),
            Some("src/main/java/com/example/Foo.java".to_string())
        );
    }
}
