#[cfg(test)]
mod tests {
    use crate::analysis::graph::*;
    use crate::analysis::test_helpers::make_file;
    use crate::core::types::ImportEdge;
    use crate::core::types::{FileNode, StructuralAnalysis};

    #[test]
    fn no_root_produces_no_import_edges() {
        let files = vec![make_file(
            "main.ts",
            "src/main.ts",
            "typescript",
            Some(StructuralAnalysis {
                functions: None,
                cls: None,
                imp: Some(vec!["./utils".to_string()]),
                co: None,
                tags: None, comment_lines: None,
            }),
        )];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, None, 5);
        assert!(gr.import_edges.is_empty(), "No root = no import edges");
    }

    #[test]
    fn call_edges_require_import() {
        let tmp = std::env::temp_dir().join("sentrux_test_call_import_gate");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![
            make_file(
                "main.py",
                "main.py",
                "python",
                Some(StructuralAnalysis {
                    functions: Some(vec![crate::core::types::FuncInfo {
                        n: "main".to_string(),
                        sl: 1, el: 5, ln: 5, cc: None, cog: None, pc: None, bh: None, d: None,
                        co: Some(vec!["helper".to_string()]), is_public: false,
                    }]),
                    cls: None,
                    imp: Some(vec![".utils".to_string()]),
                    co: None,
                    tags: None, comment_lines: None,
                }),
            ),
            make_file(
                "utils.py",
                "utils.py",
                "python",
                Some(StructuralAnalysis {
                    functions: Some(vec![crate::core::types::FuncInfo {
                        n: "helper".to_string(),
                        sl: 1, el: 3, ln: 3, cc: None, cog: None, pc: None, bh: None, d: None, co: None, is_public: false,
                    }]),
                    cls: None, imp: None, co: None, tags: None, comment_lines: None,
                }),
            ),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert_eq!(gr.import_edges.len(), 1, "expected import edge main->utils");
        assert_eq!(gr.call_edges.len(), 1, "expected 1 call edge");
        assert_eq!(gr.call_edges[0].from_file, "main.py");
        assert_eq!(gr.call_edges[0].from_func, "main");
        assert_eq!(gr.call_edges[0].to_file, "utils.py");
        assert_eq!(gr.call_edges[0].to_func, "helper");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn call_edges_blocked_without_import() {
        let files = vec![
            make_file("main.py", "main.py", "python",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["math".to_string()]),
                    co: Some(vec!["sqrt".to_string()]),
                    tags: None, comment_lines: None,
                }),
            ),
            make_file("utils.py", "utils.py", "python",
                Some(StructuralAnalysis {
                    functions: Some(vec![crate::core::types::FuncInfo {
                        n: "sqrt".to_string(),
                        sl: 1, el: 3, ln: 3, cc: None, cog: None, pc: None, bh: None, d: None, co: None, is_public: false,
                    }]),
                    cls: None, imp: None, co: None, tags: None, comment_lines: None,
                }),
            ),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, None, 5);
        assert_eq!(gr.call_edges.len(), 0, "call edge without import should be blocked");
    }

    #[test]
    fn stdlib_import_creates_no_edges() {
        let tmp = std::env::temp_dir().join("sentrux_test_stdlib_no_edge");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![
            make_file("foo.py", "src/foo.py", "python",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["math".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
            make_file("bar.py", "src/bar.py", "python",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["math".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert_eq!(gr.import_edges.len(), 0);
        assert_eq!(gr.call_edges.len(), 0);
        assert_eq!(gr.inherit_edges.len(), 0);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn stdlib_name_collision_no_false_edge() {
        let tmp = std::env::temp_dir().join("sentrux_test_stdlib_collision");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![
            make_file("foo.py", "src/foo.py", "python",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["math".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
            make_file("math.py", "src/deep/math.py", "python",
                Some(StructuralAnalysis {
                    functions: None, cls: None, imp: None, co: None, tags: None, comment_lines: None,
                }),
            ),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert_eq!(gr.import_edges.len(), 0,
            "single-segment 'import math' must NOT suffix-match to src/deep/math.py");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rust_crate_prefix_resolves() {
        let tmp = std::env::temp_dir().join("sentrux_test_rust_crate_prefix");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![
            make_file("test_capture.rs", "tests/test_capture.rs", "rust",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["beemem/capture".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
            make_file("capture.rs", "src/capture.rs", "rust", None),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert_eq!(gr.import_edges.len(), 1,
            "use beemem::capture should resolve to src/capture.rs, got {:?}", gr.import_edges);
        assert_eq!(gr.import_edges[0].from_file, "tests/test_capture.rs");
        assert_eq!(gr.import_edges[0].to_file, "src/capture.rs");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rust_crate_prefix_no_stdlib_match() {
        let tmp = std::env::temp_dir().join("sentrux_test_rust_no_stdlib");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![
            make_file("main.rs", "src/main.rs", "rust",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["std/collections/HashMap".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert_eq!(gr.import_edges.len(), 0, "std:: import should not match any project file");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rust_std_io_no_false_match_to_project_io() {
        let tmp = std::env::temp_dir().join("sentrux_test_std_io_collision");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![
            make_file("main.rs", "src/main.rs", "rust",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["std/io".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
            make_file("io.rs", "src/io.rs", "rust", None),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert_eq!(gr.import_edges.len(), 1,
            "unique io.rs match is acceptable for ambiguous std::io");

        let files2 = vec![
            make_file("test_main.rs", "tests/test_main.rs", "rust",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["std/io".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
            make_file("io.rs", "src/io.rs", "rust", None),
        ];

        let refs2: Vec<&FileNode> = files2.iter().collect();
        let gr2 = build_graphs(&refs2, Some(&tmp), 5);
        assert_eq!(gr2.import_edges.len(), 1,
            "unique io.rs from different dir still matches via suffix-index");

        let files3 = vec![
            make_file("test_main.rs", "tests/test_main.rs", "rust",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["std/io".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
            make_file("io.rs", "src/io.rs", "rust", None),
            make_file("io.rs", "lib/io.rs", "rust", None),
        ];

        let refs3: Vec<&FileNode> = files3.iter().collect();
        let gr3 = build_graphs(&refs3, Some(&tmp), 5);
        assert_eq!(gr3.import_edges.len(), 0,
            "ambiguous io (2 candidates, no dir-relative) must NOT create edge");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tier3_html_resolves_script_src() {
        let tmp = std::env::temp_dir().join("sentrux_test_tier3_html");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![
            make_file("index.html", "index.html", "html",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["app.js".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
            make_file("app.js", "app.js", "javascript", None),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert_eq!(gr.import_edges.len(), 1);
        assert_eq!(gr.import_edges[0].from_file, "index.html");
        assert_eq!(gr.import_edges[0].to_file, "app.js");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tier3_html_resolves_absolute_from_root() {
        let tmp = std::env::temp_dir().join("sentrux_test_tier3_html_abs");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![
            make_file("index.html", "webapp/index.html", "html",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["/src/style.css".to_string(), "/src/main.ts".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
            make_file("style.css", "webapp/src/style.css", "css", None),
            make_file("main.ts", "webapp/src/main.ts", "typescript", None),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert_eq!(gr.import_edges.len(), 2, "expected 2 edges, got {:?}", gr.import_edges);
        let targets: Vec<&str> = gr.import_edges.iter().map(|e| e.to_file.as_str()).collect();
        assert!(targets.contains(&"webapp/src/style.css"));
        assert!(targets.contains(&"webapp/src/main.ts"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tier3_html_absolute_fallback_to_scan_root() {
        let tmp = std::env::temp_dir().join("sentrux_test_tier3_html_fallback");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![
            make_file("index.html", "index.html", "html",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["/src/style.css".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
            make_file("style.css", "src/style.css", "css", None),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert_eq!(gr.import_edges.len(), 1);
        assert_eq!(gr.import_edges[0].to_file, "src/style.css");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tier3_css_import_resolves() {
        let tmp = std::env::temp_dir().join("sentrux_test_tier3_css");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![
            make_file("main.css", "src/main.css", "css",
                Some(StructuralAnalysis {
                    functions: None, cls: None,
                    imp: Some(vec!["../reset.css".to_string()]),
                    co: None, tags: None, comment_lines: None,
                }),
            ),
            make_file("reset.css", "reset.css", "css", None),
        ];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert_eq!(gr.import_edges.len(), 1);
        assert_eq!(gr.import_edges[0].from_file, "src/main.css");
        assert_eq!(gr.import_edges[0].to_file, "reset.css");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tier3_no_self_edges() {
        let tmp = std::env::temp_dir().join("sentrux_test_tier3_self");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![make_file("style.css", "style.css", "css",
            Some(StructuralAnalysis {
                functions: None, cls: None,
                imp: Some(vec!["./style.css".to_string()]),
                co: None, tags: None, comment_lines: None,
            }),
        )];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert!(gr.import_edges.is_empty(), "Self-imports should not produce edges");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tier3_unknown_target_no_edge() {
        let tmp = std::env::temp_dir().join("sentrux_test_tier3_unknown");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let files = vec![make_file("index.html", "index.html", "html",
            Some(StructuralAnalysis {
                functions: None, cls: None,
                imp: Some(vec!["nonexistent.js".to_string()]),
                co: None, tags: None, comment_lines: None,
            }),
        )];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, Some(&tmp), 5);
        assert!(gr.import_edges.is_empty(), "Unknown target should produce no edge");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn tier3_no_root_returns_empty() {
        let files = vec![make_file("index.html", "index.html", "html",
            Some(StructuralAnalysis {
                functions: None, cls: None,
                imp: Some(vec!["app.js".to_string()]),
                co: None, tags: None, comment_lines: None,
            }),
        )];

        let refs: Vec<&FileNode> = files.iter().collect();
        let gr = build_graphs(&refs, None, 5);
        assert!(gr.import_edges.is_empty(), "None scan_root should return empty");
    }

}
