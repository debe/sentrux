//! Tree-sitter structural parser — extracts functions, classes, imports, and calls.
//!
//! Parses source files using language-specific tree-sitter grammars and queries.
//! Results are cached (LRU, 2000 entries) by content hash to skip reparsing
//! unchanged files during incremental rescan. Thread-safe via Mutex + thread-local parsers.

mod ast_import_walker;
mod captures;
pub mod imports;
mod lang_extractors;
mod strings;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests2;
#[cfg(test)]
mod ast_import_test;


use super::lang_registry;
use self::imports::extract_bash_imports;
use self::captures::{
    classify_captures, process_func_def, process_class_def, process_import,
    ImportContext, MatchKind, ParseContext,
};
use crate::core::types::{FuncInfo, StructuralAnalysis};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor, Tree};

const CACHE_CAP: usize = 2000;

/// Unified parse cache: HashMap + insertion-order VecDeque under a single Mutex.
/// Previous design used DashMap + separate Mutex<VecDeque> which raced:
/// two threads could insert the same hash, creating duplicate CACHE_ORDER
/// entries while CACHE had only one. Eviction then removed ghost keys,
/// causing the cache to fill faster than intended. [ref:93cf32d4]
struct ParseCache {
    map: HashMap<String, StructuralAnalysis>,
    order: VecDeque<String>,
}

impl ParseCache {
    fn new() -> Self {
        Self {
            map: HashMap::with_capacity(CACHE_CAP),
            order: VecDeque::with_capacity(CACHE_CAP),
        }
    }

    fn get(&self, key: &str) -> Option<&StructuralAnalysis> {
        self.map.get(key)
    }

    fn insert(&mut self, key: String, value: StructuralAnalysis) {
        // Only insert if not already present (dedup)
        if self.map.contains_key(&key) {
            return;
        }
        // Evict oldest 10% when full
        if self.map.len() >= CACHE_CAP {
            let evict_count = CACHE_CAP / 10;
            for _ in 0..evict_count {
                if let Some(k) = self.order.pop_front() {
                    self.map.remove(&k);
                } else {
                    break;
                }
            }
        }
        self.map.insert(key.clone(), value);
        self.order.push_back(key);
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

static CACHE: std::sync::LazyLock<Mutex<ParseCache>> =
    std::sync::LazyLock::new(|| Mutex::new(ParseCache::new()));

// Thread-local parser to avoid re-creating Parser on every call
thread_local! {
    static TL_PARSER: RefCell<Parser> = RefCell::new(Parser::new());
}

/// Clear the parser cache — called on directory switch to prevent monotonic
/// growth across scan sessions. [ref:93cf32d4]
pub fn clear_cache() {
    match CACHE.lock() {
        Ok(mut cache) => cache.clear(),
        Err(poisoned) => poisoned.into_inner().clear(),
    }
}

fn content_hash(content: &[u8], lang: &str) -> u64 {
    // Use fast non-cryptographic hash (SipHash via std's DefaultHasher).
    // We only need dedup within a session, not collision resistance.
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    content.hash(&mut h);
    // Mix in length to reduce collisions for short content
    content.len().hash(&mut h);
    // Mix in language so identical content parsed with different grammars
    // (e.g., main.js vs main.ts) produces distinct cache keys. [C1 fix]
    lang.hash(&mut h);
    h.finish()
}

fn content_hash_str(content: &[u8], lang: &str) -> String {
    format!("{:016x}", content_hash(content, lang))
}

/// Accumulated state during query extraction.
struct ExtractionState {
    functions: Vec<FuncInfo>,
    func_set: HashSet<(String, u32)>,
    classes: Vec<crate::core::types::ClassInfo>,
    imports: Vec<String>,
    import_set: HashSet<String>,
    calls_raw: Vec<(String, u32)>,
    tags: Vec<String>,
    tag_set: HashSet<String>,
}

impl ExtractionState {
    fn new() -> Self {
        Self {
            functions: Vec::new(),
            func_set: HashSet::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            import_set: HashSet::new(),
            calls_raw: Vec::new(),
            tags: Vec::new(),
            tag_set: HashSet::new(),
        }
    }

    /// Dispatch a classified capture result to the appropriate handler.
    fn dispatch_match(
        &mut self,
        r: captures::CaptureResult<'_>,
        fallback_node: tree_sitter::Node<'_>,
        pctx: &ParseContext<'_>,
    ) {
        match r.match_type {
            Some(MatchKind::FuncDef) => {
                if let Some(name) = r.name_text {
                    process_func_def(
                        name, r.match_node, fallback_node,
                        pctx, &mut self.functions, &mut self.func_set,
                    );
                }
            }
            Some(MatchKind::ClassDef) => {
                process_class_def(
                    r.name_text, r.match_node, r.class_kind,
                    pctx, &mut self.classes,
                );
            }
            Some(MatchKind::Import) => {
                let ictx = ImportContext {
                    import_module_text: r.import_module_text,
                    name_text: r.name_text,
                    import_node: r.import_node,
                    match_node: r.match_node,
                };
                process_import(
                    &ictx, pctx.lang, pctx.content, &mut self.imports, &mut self.import_set,
                );
            }
            Some(MatchKind::Call) => {
                if let Some(name) = r.name_text {
                    self.calls_raw.push((name, r.call_line));
                }
            }
            None => {}
        }
    }

    /// Apply language-specific post-processing for imports.
    /// Most languages no longer need this — imports are captured directly by
    /// tree-sitter queries with @import/@import.module captures.
    /// Bash still needs text scanning because tree-sitter-bash doesn't
    /// consistently capture quoted arguments to `source` commands.
    fn post_process_imports(&mut self, content: &[u8], lang: &str) {
        if lang == "bash" {
            extract_bash_imports(content, &mut self.imports, &mut self.import_set);
        }
    }

    /// Convert into a StructuralAnalysis, distributing calls to functions.
    fn into_structural_analysis(mut self) -> StructuralAnalysis {
        let module_calls = distribute_calls_to_functions(&self.calls_raw, &mut self.functions);
        StructuralAnalysis {
            functions: if self.functions.is_empty() { None } else { Some(self.functions) },
            cls: if self.classes.is_empty() { None } else { Some(self.classes) },
            imp: if self.imports.is_empty() { None } else { Some(self.imports) },
            co: if module_calls.is_empty() { None } else { Some(module_calls) },
            tags: if self.tags.is_empty() { None } else { Some(self.tags) },
        }
    }
}

/// Extract structural analysis from a parsed syntax tree using language-specific queries.
pub(crate) fn extract_with_queries(
    tree: &Tree,
    content: &[u8],
    query: &Query,
    lang: &str,
) -> StructuralAnalysis {
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(query, tree.root_node(), content);
    let capture_names = query.capture_names();
    let mut state = ExtractionState::new();
    let pctx = ParseContext { content, lang };

    while let Some(m) = matches.next() {
        if m.captures.is_empty() {
            continue;
        }
        let r = classify_captures(
            m.captures, capture_names, content,
            &mut state.imports, &mut state.import_set, &mut state.tags, &mut state.tag_set,
        );
        state.dispatch_match(r, m.captures[0].node, &pctx);
    }

    state.post_process_imports(content, lang);
    state.into_structural_analysis()
}


/// Distribute calls to their containing functions.
/// Each call is assigned to the innermost function whose [sl, el] range
/// contains the call's source line. Calls outside any function go to
/// the file-level `co` (module-level code). Returns module-level calls.
/// Find the index of the innermost function containing a given line.
/// Uses binary search on a pre-sorted (by start line) slice for O(log F) lookup.
fn find_containing_function(sorted_indices: &[(u32, u32, usize)], line: u32) -> Option<usize> {
    // sorted_indices: Vec of (sl, el, original_index), sorted by sl ascending.
    // Binary search for the rightmost function whose sl <= line.
    let pos = sorted_indices.partition_point(|&(sl, _, _)| sl <= line);
    if pos == 0 {
        return None;
    }
    // Walk backwards from pos-1 to find the innermost (highest sl) function containing line.
    // Since sorted by sl, the first match walking back from pos-1 is the innermost.
    for i in (0..pos).rev() {
        let (sl, el, orig_idx) = sorted_indices[i];
        if line >= sl && line <= el {
            return Some(orig_idx);
        }
    }
    None
}

/// Dedup call lists per function and assign to func.co.
fn assign_deduped_calls(functions: &mut [FuncInfo], func_calls: &mut [Vec<String>]) {
    for (i, func) in functions.iter_mut().enumerate() {
        if !func_calls[i].is_empty() {
            let mut seen = HashSet::new();
            func_calls[i].retain(|c| seen.insert(c.clone()));
            func.co = Some(std::mem::take(&mut func_calls[i]));
        }
    }
}

fn distribute_calls_to_functions(
    calls_raw: &[(String, u32)],
    functions: &mut [FuncInfo],
) -> Vec<String> {
    // Build sorted index for O(log F) lookup per call site.
    let mut sorted_indices: Vec<(u32, u32, usize)> = functions
        .iter()
        .enumerate()
        .map(|(i, f)| (f.sl, f.el, i))
        .collect();
    sorted_indices.sort_unstable_by_key(|&(sl, _, _)| sl);

    let mut func_calls: Vec<Vec<String>> = vec![Vec::new(); functions.len()];
    let mut module_calls: Vec<String> = Vec::new();
    let mut module_call_set: HashSet<String> = HashSet::new();

    for (call_name, line) in calls_raw {
        match find_containing_function(&sorted_indices, *line) {
            Some(idx) => func_calls[idx].push(call_name.clone()),
            None => {
                if module_call_set.insert(call_name.clone()) {
                    module_calls.push(call_name.clone());
                }
            }
        }
    }

    assign_deduped_calls(functions, &mut func_calls);
    module_calls
}


/// Parse a file and extract structural analysis. Cached by content hash.
/// Uses thread-local Parser and registry-based query extraction.
/// Cache uses a single Mutex<ParseCache> — no CACHE/CACHE_ORDER split race. [ref:93cf32d4]
pub fn parse_file(path: &str, lang: &str, max_parse_size: usize) -> Option<StructuralAnalysis> {
    // Check file size BEFORE reading to prevent OOM on large binaries
    let file_size = std::fs::metadata(path).ok()?.len() as usize;
    if file_size > max_parse_size * 1024 {
        return None;
    }
    let content = std::fs::read(path).ok()?;

    let hash = content_hash_str(&content, lang);

    // Check cache (short lock, clone inside to release quickly)
    {
        let cache = match CACHE.lock() {
            Ok(c) => c,
            Err(p) => p.into_inner(),
        };
        if let Some(cached) = cache.get(&hash) {
            return Some(cached.clone());
        }
    }

    // Look up language config from registry (plugins override built-in)
    let (grammar, query) = lang_registry::get_grammar_and_query(lang)?;

    // Use thread-local parser
    let tree = TL_PARSER.with(|parser_cell| {
        let mut parser = parser_cell.borrow_mut();
        if let Err(e) = parser.set_language(grammar) {
            eprintln!("[parser] set_language failed for {}: {}", lang, e);
            return None;
        }
        parser.parse(&content, None)
    })?;

    let sa = extract_with_queries(&tree, &content, query, lang);

    // Insert under lock — single data structure, no race between map and order.
    // ParseCache::insert handles dedup + eviction atomically.
    {
        let mut cache = match CACHE.lock() {
            Ok(c) => c,
            Err(p) => {
                eprintln!("[parser] cache mutex poisoned, recovering");
                p.into_inner()
            }
        };
        cache.insert(hash, sa.clone());
    }

    Some(sa)
}

/// Parse all files in parallel using rayon, returning (rel_path, analysis) pairs.
pub fn parse_files_batch(
    files: &[(String, String, String)], // (abs_path, rel_path, lang)
    max_parse_size: usize,
) -> Vec<(String, StructuralAnalysis)> {
    parse_files_batch_with_progress(files, max_parse_size, None)
}

/// Shared progress counter for parse_files_batch.
/// The scanner spawns the parse on a thread and polls this from the main scan thread.
pub struct ParseProgress {
    /// Number of files parsed so far (updated atomically by worker threads)
    pub done: std::sync::atomic::AtomicUsize,
    /// Total number of files to parse
    pub total: usize,
}

/// Parse all files in parallel via rayon, updating `progress` atomically if provided.
/// Each file is parsed independently and results are collected into a Vec.
pub fn parse_files_batch_with_progress(
    files: &[(String, String, String)],
    max_parse_size: usize,
    progress: Option<&ParseProgress>,
) -> Vec<(String, StructuralAnalysis)> {
    use rayon::prelude::*;

    let input_count = files.len();
    let failed = std::sync::atomic::AtomicUsize::new(0);
    let result: Vec<(String, StructuralAnalysis)> = files
        .par_iter()
        .filter_map(|(abs_path, rel_path, lang)| {
            let result = parse_file(abs_path, lang, max_parse_size)
                .map(|sa| (rel_path.clone(), sa));
            if result.is_none() {
                failed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(p) = progress {
                p.done.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            result
        })
        .collect();
    let fail_count = failed.load(std::sync::atomic::Ordering::Relaxed);
    if fail_count > 0 {
        eprintln!("[parser] {}/{} files failed to parse (too large or binary)", fail_count, input_count);
    }
    result
}

/// Parse raw bytes without file I/O or cache. Used by tests in parser_tests.
#[cfg(test)]
pub(crate) fn parse_bytes(content: &[u8], lang: &str) -> Option<StructuralAnalysis> {
    let (grammar, query) = lang_registry::get_grammar_and_query(lang)?;
    let tree = TL_PARSER.with(|parser_cell| {
        let mut parser = parser_cell.borrow_mut();
        parser.set_language(grammar).ok()?;
        parser.parse(content, None)
    })?;
    Some(extract_with_queries(&tree, content, query, lang))
}
