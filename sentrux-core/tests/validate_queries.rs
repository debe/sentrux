//! Integration test: validate tree-sitter queries against actual grammars.
//! Also probes alternative query patterns to find what works.

fn load_grammar(name: &str, symbol_override: Option<&str>) -> tree_sitter::Language {
    let home = std::env::var("HOME").unwrap();
    let grammar_path = format!("{home}/.sentrux/plugins/{name}/grammars/darwin-arm64.dylib");
    let symbol_name = match symbol_override {
        Some(s) => format!("tree_sitter_{s}"),
        None => format!("tree_sitter_{}", name.replace('-', "_")),
    };
    unsafe {
        let lib = libloading::Library::new(&grammar_path).unwrap();
        let func: libloading::Symbol<unsafe extern "C" fn() -> tree_sitter::Language> =
            lib.get(symbol_name.as_bytes()).unwrap();
        let lang = func();
        std::mem::forget(lib);
        lang
    }
}

#[test]
fn probe_nim() {
    let lang = load_grammar("nim", None);
    let tests = &[
        "(routine (ident) @name) @definition.function",
        "(routine\n  name: (ident) @name) @definition.function",
        // Maybe routine doesn't have ident as direct child
        "(routine) @definition.function",
    ];
    for q in tests {
        match tree_sitter::Query::new(&lang, q) {
            Ok(_) => eprintln!("OK: {}", q.replace('\n', "\\n")),
            Err(e) => eprintln!("FAIL[{:?}]: {}", e.kind, q.replace('\n', "\\n")),
        }
    }
}

#[test]
fn probe_ocaml() {
    let lang = load_grammar("ocaml", None);
    let tests = &[
        // Current (fails)
        "(module_definition\n  (module_binding\n    name: (module_name) @name)) @definition.module",
        // Without field name
        "(module_definition\n  (module_binding\n    (module_name) @name)) @definition.module",
        // Just module_definition
        "(module_definition) @definition.module",
    ];
    for q in tests {
        match tree_sitter::Query::new(&lang, q) {
            Ok(_) => eprintln!("OK: {}", q.replace('\n', "\\n")),
            Err(e) => eprintln!("FAIL[{:?}]: {}", e.kind, q.replace('\n', "\\n")),
        }
    }
}

#[test]
fn probe_nix() {
    let lang = load_grammar("nix", None);
    let tests = &[
        "(inherit\n  (identifier) @name) @definition.function",
        "(inherit\n  (inherited_attrs\n    (identifier) @name)) @definition.function",
        "(inherit\n  (attrs_inherited\n    (identifier) @name)) @definition.function",
        "(inherit) @definition.function",
        // Try identifier as attr
        "(inherit\n  attr: (identifier) @name) @definition.function",
    ];
    for q in tests {
        match tree_sitter::Query::new(&lang, q) {
            Ok(_) => eprintln!("OK: {}", q.replace('\n', "\\n")),
            Err(e) => eprintln!("FAIL[{:?}]: {}", e.kind, q.replace('\n', "\\n")),
        }
    }
}

#[test]
fn probe_fsharp() {
    let lang = load_grammar("fsharp", None);
    let tests = &[
        "(type_definition\n  (type_name\n    (long_identifier) @name)) @definition.class",
        "(type_definition\n  (type_name) @name) @definition.class",
        "(type_definition) @definition.class",
        // Check if type_name accepts long_identifier
        "(type_name\n  (long_identifier) @name) @definition.class",
        "(type_name) @definition.class",
    ];
    for q in tests {
        match tree_sitter::Query::new(&lang, q) {
            Ok(_) => eprintln!("OK: {}", q.replace('\n', "\\n")),
            Err(e) => eprintln!("FAIL[{:?}]: {}", e.kind, q.replace('\n', "\\n")),
        }
    }
}

#[test]
fn probe_powershell() {
    let lang = load_grammar("powershell", None);
    let tests = &[
        "(enum_statement\n  (type_identifier) @name) @definition.class",
        "(enum_statement\n  name: (type_identifier) @name) @definition.class",
        "(enum_statement) @definition.class",
        // Check what enum_statement looks like
        "(class_statement\n  (type_identifier) @name) @definition.class",
        "(class_statement\n  name: (type_identifier) @name) @definition.class",
        "(function_statement\n  (function_name) @name) @definition.function",
        "(function_statement\n  name: (function_name) @name) @definition.function",
    ];
    for q in tests {
        match tree_sitter::Query::new(&lang, q) {
            Ok(_) => eprintln!("OK: {}", q.replace('\n', "\\n")),
            Err(e) => eprintln!("FAIL[{:?}]: {}", e.kind, q.replace('\n', "\\n")),
        }
    }
}

#[test]
fn probe_julia() {
    let lang = load_grammar("julia", None);
    let tests = &[
        "(function_definition\n  name: (identifier) @name) @definition.function",
        "(function_definition\n  (identifier) @name) @definition.function",
        "(function_definition) @definition.function",
    ];
    for q in tests {
        match tree_sitter::Query::new(&lang, q) {
            Ok(_) => eprintln!("OK: {}", q.replace('\n', "\\n")),
            Err(e) => eprintln!("FAIL[{:?}]: {}", e.kind, q.replace('\n', "\\n")),
        }
    }
}

#[test]
fn probe_groovy() {
    let lang = load_grammar("groovy", None);
    let tests = &[
        "(function_definition\n  name: (identifier) @name) @definition.function",
        "(function_definition\n  (identifier) @name) @definition.function",
        "(function_definition) @definition.function",
        // Maybe it's method_declaration?
        "(method_declaration\n  name: (identifier) @name) @definition.function",
        "(method_declaration\n  (identifier) @name) @definition.function",
        "(class_definition\n  name: (identifier) @name) @definition.class",
        "(class_definition\n  (identifier) @name) @definition.class",
        "(function_declaration\n  name: (identifier) @name) @definition.function",
        "(function_declaration\n  (identifier) @name) @definition.function",
    ];
    for q in tests {
        match tree_sitter::Query::new(&lang, q) {
            Ok(_) => eprintln!("OK: {}", q.replace('\n', "\\n")),
            Err(e) => eprintln!("FAIL[{:?}]: {}", e.kind, q.replace('\n', "\\n")),
        }
    }
}

#[test]
fn probe_objc() {
    let lang = load_grammar("objective-c", Some("objc"));
    let tests = &[
        "(method_declaration\n  selector: (keyword_selector\n    (keyword_declarator\n      keyword: (identifier) @name))) @definition.function",
        // Without keyword_selector
        "(method_declaration\n  selector: (_) @name) @definition.function",
        "(method_declaration) @definition.function",
        // Check if it's method_definition instead
        "(method_definition) @definition.function",
        "(method_definition\n  selector: (_) @name) @definition.function",
    ];
    for q in tests {
        match tree_sitter::Query::new(&lang, q) {
            Ok(_) => eprintln!("OK: {}", q.replace('\n', "\\n")),
            Err(e) => eprintln!("FAIL[{:?}]: {}", e.kind, q.replace('\n', "\\n")),
        }
    }
}
