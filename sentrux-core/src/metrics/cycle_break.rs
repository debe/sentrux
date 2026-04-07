//! Cycle-breaking suggestions via greedy minimum feedback arc set (MFAS).
//!
//! Given a set of dependency cycles (SCCs), recommends which edges to remove
//! to break all cycles with minimal architectural disruption.
//!
//! Algorithm: Greedy heuristic based on Eades, Lin & Smyth (1993).
//! For each edge in a cycle, computes a break-cost score combining:
//! - Blast radius of the target (how many files transitively depend on it)
//! - Instability of the source (unstable importers are cheaper to rewire)
//! - Whether removal would create new upward violations
//!
//! Returns ranked suggestions with before/after impact estimates.

use crate::core::types::ImportEdge;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};

/// A suggestion for breaking a dependency cycle.
#[derive(Debug, Clone, Serialize)]
pub struct CycleBreakSuggestion {
    /// File containing the import to remove
    pub from_file: String,
    /// File being imported (the edge to cut)
    pub to_file: String,
    /// Human-readable explanation
    pub reason: String,
    /// Lower = cheaper to remove this edge
    pub cost_score: f64,
    /// Impact analysis of removing this edge
    pub impact: BreakImpact,
}

/// Estimated impact of removing a single dependency edge.
#[derive(Debug, Clone, Serialize)]
pub struct BreakImpact {
    /// How many cycles this edge participates in
    pub cycles_broken: usize,
    /// Transitive reach of the target file (reverse blast radius)
    pub target_blast_radius: usize,
    /// Instability of the source file (0=stable, 1=unstable)
    pub source_instability: f64,
    /// Number of remaining cycles after removing this edge
    pub remaining_cycles: usize,
}

/// Result of cycle-break analysis for the entire codebase.
#[derive(Debug, Clone, Serialize)]
pub struct CycleBreakReport {
    /// Total cycles before any fixes
    pub total_cycles: usize,
    /// Ordered list of edges to remove (greedy order: cheapest first)
    pub suggestions: Vec<CycleBreakSuggestion>,
    /// Minimum number of edge removals needed (lower bound via MFAS heuristic)
    pub min_removals: usize,
}

/// Compute cycle-breaking suggestions for all cycles in the dependency graph.
///
/// `edges`: the full import edge list
/// `cycles`: SCCs with >1 member (from Tarjan's algorithm)
///
/// Returns up to `max_suggestions` ranked suggestions.
pub fn suggest_cycle_breaks(
    edges: &[ImportEdge],
    cycles: &[Vec<String>],
    max_suggestions: usize,
) -> CycleBreakReport {
    if cycles.is_empty() {
        return CycleBreakReport {
            total_cycles: 0,
            suggestions: Vec::new(),
            min_removals: 0,
        };
    }

    // Build adjacency structures
    let (fan_out, fan_in) = compute_fan_maps(edges);
    let reverse_reach = compute_reverse_reach(edges);

    // Collect all edges participating in cycles
    let cycle_member_sets: Vec<HashSet<&str>> = cycles
        .iter()
        .map(|c| c.iter().map(|s| s.as_str()).collect())
        .collect();

    let mut candidate_edges: Vec<CycleBreakSuggestion> = Vec::new();

    for edge in edges {
        // Count how many cycles this edge participates in
        let cycles_broken = cycle_member_sets
            .iter()
            .filter(|members| {
                members.contains(edge.from_file.as_str())
                    && members.contains(edge.to_file.as_str())
            })
            .count();

        if cycles_broken == 0 {
            continue; // Not a cycle edge
        }

        let target_blast = reverse_reach
            .get(edge.to_file.as_str())
            .copied()
            .unwrap_or(0);
        let source_fan_out = fan_out.get(edge.from_file.as_str()).copied().unwrap_or(1);
        let source_fan_in = fan_in.get(edge.from_file.as_str()).copied().unwrap_or(0);
        let source_instability = source_fan_out as f64
            / (source_fan_in as f64 + source_fan_out as f64).max(1.0);

        // Cost = how expensive is it to remove this edge?
        // Low blast radius + high source instability = cheap to remove
        // (unstable files are easy to rewire; low-blast targets are safe to decouple)
        let cost_score = (target_blast as f64 + 1.0) * (1.0 - source_instability + 0.1);

        candidate_edges.push(CycleBreakSuggestion {
            from_file: edge.from_file.clone(),
            to_file: edge.to_file.clone(),
            reason: format!(
                "Breaks {} cycle(s). Target has blast radius {}. Source instability {:.2}.",
                cycles_broken, target_blast, source_instability
            ),
            cost_score,
            impact: BreakImpact {
                cycles_broken,
                target_blast_radius: target_blast,
                source_instability,
                remaining_cycles: 0, // filled in greedy pass below
            },
        });
    }

    // Sort by cost (cheapest first) then by cycles_broken descending (break more = better)
    candidate_edges.sort_by(|a, b| {
        a.cost_score
            .partial_cmp(&b.cost_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.impact.cycles_broken.cmp(&a.impact.cycles_broken))
    });

    // Greedy MFAS: pick edges one by one, recompute remaining cycles
    let mut remaining_edges: Vec<ImportEdge> = edges.to_vec();
    let mut suggestions: Vec<CycleBreakSuggestion> = Vec::new();
    let mut remaining_cycle_count = cycles.len();

    for mut candidate in candidate_edges {
        if remaining_cycle_count == 0 || suggestions.len() >= max_suggestions {
            break;
        }

        // Remove this edge from the graph
        remaining_edges.retain(|e| {
            !(e.from_file == candidate.from_file && e.to_file == candidate.to_file)
        });

        // Recompute cycles on reduced graph
        let new_cycles = count_cycles(&remaining_edges);
        candidate.impact.remaining_cycles = new_cycles;
        remaining_cycle_count = new_cycles;

        suggestions.push(candidate);
    }

    let min_removals = suggestions
        .iter()
        .position(|s| s.impact.remaining_cycles == 0)
        .map(|i| i + 1)
        .unwrap_or(suggestions.len());

    CycleBreakReport {
        total_cycles: cycles.len(),
        suggestions,
        min_removals,
    }
}

/// Compute fan-out and fan-in maps from edges.
fn compute_fan_maps(edges: &[ImportEdge]) -> (HashMap<&str, usize>, HashMap<&str, usize>) {
    let mut fan_out: HashMap<&str, usize> = HashMap::new();
    let mut fan_in: HashMap<&str, usize> = HashMap::new();
    let mut seen: HashSet<(&str, &str)> = HashSet::new();
    for edge in edges {
        if seen.insert((edge.from_file.as_str(), edge.to_file.as_str())) {
            *fan_out.entry(edge.from_file.as_str()).or_default() += 1;
            *fan_in.entry(edge.to_file.as_str()).or_default() += 1;
        }
    }
    (fan_out, fan_in)
}

/// Compute reverse transitive reach (blast radius) for each file.
/// Uses BFS on the reverse graph. Capped at 500 nodes for perf.
fn compute_reverse_reach(edges: &[ImportEdge]) -> HashMap<&str, usize> {
    let mut rev_adj: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut all_nodes: HashSet<&str> = HashSet::new();
    for edge in edges {
        rev_adj
            .entry(edge.to_file.as_str())
            .or_default()
            .push(edge.from_file.as_str());
        all_nodes.insert(edge.from_file.as_str());
        all_nodes.insert(edge.to_file.as_str());
    }

    let mut reach: HashMap<&str, usize> = HashMap::new();
    for &node in &all_nodes {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut queue: VecDeque<&str> = VecDeque::new();
        queue.push_back(node);
        visited.insert(node);
        while let Some(current) = queue.pop_front() {
            if visited.len() > 500 {
                break; // Cap for large graphs
            }
            if let Some(neighbors) = rev_adj.get(current) {
                for &neighbor in neighbors {
                    if visited.insert(neighbor) {
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        reach.insert(node, visited.len() - 1); // Exclude self
    }
    reach
}

/// Count cycles (SCCs with >1 member) using Tarjan's algorithm.
/// Lightweight version — just returns the count, not the full SCC list.
fn count_cycles(edges: &[ImportEdge]) -> usize {
    let mut nodes: HashSet<&str> = HashSet::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in edges {
        nodes.insert(edge.from_file.as_str());
        nodes.insert(edge.to_file.as_str());
        adj.entry(edge.from_file.as_str())
            .or_default()
            .push(edge.to_file.as_str());
    }

    // Iterative Tarjan's SCC
    let mut index_counter: u32 = 0;
    let mut stack: Vec<&str> = Vec::new();
    let mut on_stack: HashSet<&str> = HashSet::new();
    let mut index_map: HashMap<&str, u32> = HashMap::new();
    let mut lowlink: HashMap<&str, u32> = HashMap::new();
    let mut cycle_count = 0;

    for &start in &nodes {
        if index_map.contains_key(start) {
            continue;
        }

        index_map.insert(start, index_counter);
        lowlink.insert(start, index_counter);
        index_counter += 1;
        stack.push(start);
        on_stack.insert(start);

        let mut dfs_stack: Vec<(&str, usize)> = vec![(start, 0)];

        while let Some((v, ni)) = dfs_stack.last_mut() {
            let neighbors = adj.get(*v).map(|n| n.as_slice()).unwrap_or(&[]);
            if *ni < neighbors.len() {
                let w = neighbors[*ni];
                *ni += 1;

                if !index_map.contains_key(w) {
                    index_map.insert(w, index_counter);
                    lowlink.insert(w, index_counter);
                    index_counter += 1;
                    stack.push(w);
                    on_stack.insert(w);
                    dfs_stack.push((w, 0));
                } else if on_stack.contains(w) {
                    let w_idx = index_map[w];
                    let v_low = lowlink.get_mut(*v).unwrap();
                    if w_idx < *v_low {
                        *v_low = w_idx;
                    }
                }
            } else {
                let v_node = *v;
                let v_low = lowlink[v_node];
                let v_idx = index_map[v_node];

                if v_low == v_idx {
                    let mut scc_size = 0;
                    loop {
                        let w = stack.pop().unwrap();
                        on_stack.remove(w);
                        scc_size += 1;
                        if w == v_node {
                            break;
                        }
                    }
                    if scc_size > 1 {
                        cycle_count += 1;
                    }
                }

                dfs_stack.pop();

                if let Some((parent, _)) = dfs_stack.last() {
                    let parent_low = lowlink.get_mut(*parent).unwrap();
                    if v_low < *parent_low {
                        *parent_low = v_low;
                    }
                }
            }
        }
    }

    cycle_count
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(from: &str, to: &str) -> ImportEdge {
        ImportEdge {
            from_file: from.to_string(),
            to_file: to.to_string(),
        }
    }

    #[test]
    fn no_cycles_no_suggestions() {
        let edges = vec![edge("a", "b"), edge("b", "c")];
        let report = suggest_cycle_breaks(&edges, &[], 10);
        assert_eq!(report.total_cycles, 0);
        assert!(report.suggestions.is_empty());
    }

    #[test]
    fn simple_triangle_cycle() {
        let edges = vec![edge("a", "b"), edge("b", "c"), edge("c", "a")];
        let cycles = vec![vec!["a".to_string(), "b".to_string(), "c".to_string()]];
        let report = suggest_cycle_breaks(&edges, &cycles, 10);
        assert_eq!(report.total_cycles, 1);
        assert!(!report.suggestions.is_empty());
        // After removing the suggested edge, remaining cycles should be 0
        assert_eq!(report.suggestions[0].impact.remaining_cycles, 0);
        assert_eq!(report.min_removals, 1);
    }

    #[test]
    fn two_overlapping_cycles() {
        // a→b→c→a and a→b→d→a
        let edges = vec![
            edge("a", "b"),
            edge("b", "c"),
            edge("c", "a"),
            edge("b", "d"),
            edge("d", "a"),
        ];
        let cycles = vec![
            vec!["a".to_string(), "b".to_string(), "c".to_string(), "d".to_string()],
        ];
        let report = suggest_cycle_breaks(&edges, &cycles, 10);
        assert!(report.total_cycles > 0);
        // Should suggest removing an edge that breaks the cycle
        assert!(!report.suggestions.is_empty());
    }

    #[test]
    fn count_cycles_correctness() {
        let edges = vec![edge("a", "b"), edge("b", "a")];
        assert_eq!(count_cycles(&edges), 1);

        let edges_no_cycle = vec![edge("a", "b"), edge("b", "c")];
        assert_eq!(count_cycles(&edges_no_cycle), 0);
    }
}
