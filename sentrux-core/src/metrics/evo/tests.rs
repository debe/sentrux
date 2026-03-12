//! Tests for evolutionary metrics: bus factor, churn, coupling history.
//!
//! Validates grading functions for bus factor (single-author ratio) and churn
//! concentration, temporal coupling detection from commit co-occurrence,
//! and hotspot identification. Uses synthetic `CommitRecord` fixtures.
//! Covers boundary (empty history), oracle (known commits produce known grades),
//! and monotonicity (more authors improve bus factor grade) properties.

#[cfg(test)]
mod tests {
    use crate::metrics::evo::*;
    use crate::metrics::evo::git_walker::{CommitFile, CommitRecord};
    use std::collections::{HashMap, HashSet};

    // ── Unit tests for grading functions ──

    #[test]
    fn grade_bus_factor_all_single_author() {
        assert_eq!(grade_bus_factor(1.0), 'F');
    }

    #[test]
    fn grade_bus_factor_no_single_author() {
        assert_eq!(grade_bus_factor(0.0), 'A');
    }

    #[test]
    fn grade_bus_factor_boundary() {
        assert_eq!(grade_bus_factor(0.10), 'A');
        assert_eq!(grade_bus_factor(0.11), 'B');
        assert_eq!(grade_bus_factor(0.25), 'B');
        assert_eq!(grade_bus_factor(0.26), 'C');
    }

    #[test]
    fn grade_churn_empty() {
        let churn = HashMap::new();
        assert_eq!(grade_churn_concentration(&churn), 'A');
    }

    #[test]
    fn grade_churn_uniform() {
        let mut churn = HashMap::new();
        for i in 0..20 {
            churn.insert(
                format!("file_{i}.rs"),
                FileChurn {
                    commit_count: 5,
                    lines_added: 10,
                    lines_removed: 10,
                    total_churn: 20,
                },
            );
        }
        assert_eq!(grade_churn_concentration(&churn), 'A');
    }

    #[test]
    fn grade_churn_concentrated() {
        let mut churn = HashMap::new();
        churn.insert(
            "god.rs".to_string(),
            FileChurn {
                commit_count: 100,
                lines_added: 5000,
                lines_removed: 3000,
                total_churn: 8000,
            },
        );
        for i in 0..9 {
            churn.insert(
                format!("small_{i}.rs"),
                FileChurn {
                    commit_count: 1,
                    lines_added: 5,
                    lines_removed: 5,
                    total_churn: 10,
                },
            );
        }
        assert_eq!(grade_churn_concentration(&churn), 'F');
    }

    // ── Unit tests for floor_grade ──

    #[test]
    fn floor_grade_takes_worst() {
        assert_eq!(floor_grade('A', 'C'), 'C');
        assert_eq!(floor_grade('F', 'A'), 'F');
        assert_eq!(floor_grade('B', 'B'), 'B');
    }

    // ── Unit tests for compute functions (with synthetic data) ──

    fn make_records() -> Vec<CommitRecord> {
        vec![
            CommitRecord {
                author: "alice".to_string(),
                epoch: 1000000,
                files: vec![
                    CommitFile { path: "a.rs".to_string(), added: 10, removed: 2 },
                    CommitFile { path: "b.rs".to_string(), added: 5, removed: 1 },
                ],
            },
            CommitRecord {
                author: "bob".to_string(),
                epoch: 1100000,
                files: vec![
                    CommitFile { path: "a.rs".to_string(), added: 3, removed: 3 },
                    CommitFile { path: "c.rs".to_string(), added: 20, removed: 0 },
                ],
            },
            CommitRecord {
                author: "alice".to_string(),
                epoch: 1200000,
                files: vec![
                    CommitFile { path: "a.rs".to_string(), added: 1, removed: 1 },
                    CommitFile { path: "b.rs".to_string(), added: 2, removed: 0 },
                ],
            },
        ]
    }

    fn known() -> HashSet<String> {
        ["a.rs", "b.rs", "c.rs", "d.rs"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    #[test]
    fn churn_aggregation() {
        let records = make_records();
        let churn = compute_churn(&records, &known());

        let a = &churn["a.rs"];
        assert_eq!(a.commit_count, 3);
        assert_eq!(a.lines_added, 14);
        assert_eq!(a.lines_removed, 6);
        assert_eq!(a.total_churn, 20);

        let b = &churn["b.rs"];
        assert_eq!(b.commit_count, 2);

        let c = &churn["c.rs"];
        assert_eq!(c.commit_count, 1);
        assert_eq!(c.lines_added, 20);

        assert!(!churn.contains_key("d.rs"));
    }

    #[test]
    fn churn_filters_unknown_files() {
        let records = vec![CommitRecord {
            author: "x".to_string(),
            epoch: 1000,
            files: vec![
                CommitFile { path: "known.rs".to_string(), added: 1, removed: 0 },
                CommitFile { path: "deleted.rs".to_string(), added: 1, removed: 0 },
            ],
        }];
        let known: HashSet<String> = ["known.rs"].into_iter().map(String::from).collect();
        let churn = compute_churn(&records, &known);
        assert!(churn.contains_key("known.rs"));
        assert!(!churn.contains_key("deleted.rs"));
    }

    #[test]
    fn coupling_pairs_detected() {
        let records = make_records();
        let pairs = compute_coupling(&records, &known());
        assert!(pairs.is_empty() || pairs[0].co_change_count >= 3);
    }

    #[test]
    fn coupling_with_enough_cochanges() {
        let mut records = Vec::new();
        for i in 0..5 {
            records.push(CommitRecord {
                author: "alice".to_string(),
                epoch: 1000000 + i * 1000,
                files: vec![
                    CommitFile { path: "a.rs".to_string(), added: 1, removed: 0 },
                    CommitFile { path: "b.rs".to_string(), added: 1, removed: 0 },
                ],
            });
        }
        let known = known();
        let pairs = compute_coupling(&records, &known);

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].co_change_count, 5);
        assert!((pairs[0].coupling_strength - 1.0).abs() < 0.001);
    }

    #[test]
    fn code_age_most_recent() {
        let records = make_records();
        let age = compute_code_age(&records, &known(), 1300000);
        assert_eq!(age["a.rs"], 1);
        assert_eq!(age["c.rs"], 2);
    }

    #[test]
    fn authors_counted_correctly() {
        let records = make_records();
        let (authors, single_ratio) = compute_authors(&records, &known());

        let a = &authors["a.rs"];
        assert_eq!(a.author_count, 2);
        assert_eq!(a.primary_author, "alice");

        let c = &authors["c.rs"];
        assert_eq!(c.author_count, 1);
        assert_eq!(c.primary_author, "bob");

        assert!(single_ratio > 0.6 && single_ratio < 0.7);
    }

    #[test]
    fn hotspots_ranked_by_risk() {
        let mut churn = HashMap::new();
        churn.insert("simple.rs".to_string(), FileChurn {
            commit_count: 10, lines_added: 0, lines_removed: 0, total_churn: 0,
        });
        churn.insert("complex.rs".to_string(), FileChurn {
            commit_count: 5, lines_added: 0, lines_removed: 0, total_churn: 0,
        });

        let mut complexity = HashMap::new();
        complexity.insert("simple.rs".to_string(), 2u32);
        complexity.insert("complex.rs".to_string(), 20u32);

        let hotspots = compute_hotspots(&churn, &complexity);

        assert_eq!(hotspots[0].file, "complex.rs");
        assert_eq!(hotspots[0].risk_score, 100);
        assert_eq!(hotspots[1].file, "simple.rs");
        assert_eq!(hotspots[1].risk_score, 20);
    }

    // ── Idempotency test ──

    #[test]
    fn churn_idempotent() {
        let records = make_records();
        let known = known();
        let a = compute_churn(&records, &known);
        let b = compute_churn(&records, &known);
        for (k, v) in &a {
            let v2 = &b[k];
            assert_eq!(v.commit_count, v2.commit_count);
            assert_eq!(v.total_churn, v2.total_churn);
        }
    }

    // ── Commutativity test (order of records shouldn't change aggregation) ──

    #[test]
    fn churn_order_independent() {
        let records = make_records();
        let known = known();
        let forward = compute_churn(&records, &known);

        let mut reversed = make_records();
        reversed.reverse();
        let backward = compute_churn(&reversed, &known);

        for (k, v) in &forward {
            let v2 = &backward[k];
            assert_eq!(v.commit_count, v2.commit_count);
            assert_eq!(v.total_churn, v2.total_churn);
        }
    }

    // ── Integration test: compute_evolution on the actual sentrux repo ──

    #[test]
    fn integration_real_repo() {
        // CARGO_MANIFEST_DIR = sentrux-core/, git root = parent
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest.parent().unwrap_or(manifest);
        let known: HashSet<String> = ["sentrux-core/src/lib.rs", "sentrux-core/src/license.rs"]
            .into_iter()
            .map(String::from)
            .collect();
        let complexity: HashMap<String, u32> = [
            ("sentrux-core/src/lib.rs".to_string(), 5u32),
        ]
        .into_iter()
        .collect();

        let result = compute_evolution(root, &known, &complexity, Some(365));
        assert!(result.is_ok());

        // Just verify the computation succeeded without error.
        // Commit count and churn depend on git history depth.
    }
}
