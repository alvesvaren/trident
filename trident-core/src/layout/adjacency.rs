// Adjacency and edge weight computation for graph-driven layout.
//
// Builds data structures that describe node connectivity, used to:
// 1. Choose placement order (most-connected-to-placed first)
// 2. Generate candidate positions (near connected nodes)
// 3. Score candidates (minimize edge lengths)
// 4. Bias placement (external-facing nodes near boundaries)

use std::collections::HashMap;
use crate::parser::{Diagram, GroupId, ClassId};

/// Edge weight info for a single node.
#[derive(Debug, Clone, Default)]
pub struct NodeWeights {
    /// Count of edges to nodes in the same group.
    pub w_in: usize,
    /// Count of edges to nodes outside this group.
    pub w_out: usize,
}

impl NodeWeights {
    /// External ratio: fraction of edges that go outside the group.
    /// Returns 0.0 if no edges.
    pub fn external_ratio(&self) -> f32 {
        let total = self.w_in + self.w_out;
        if total == 0 {
            0.0
        } else {
            self.w_out as f32 / total as f32
        }
    }
}

/// Adjacency information for layout.
#[derive(Debug, Clone)]
pub struct Adjacency {
    /// For each class, list of (neighbor_class, edge_count).
    /// Edges are counted bidirectionally (both from→to and to→from).
    pub neighbors: HashMap<ClassId, Vec<(ClassId, usize)>>,

    /// Total degree (edge count) per class.
    pub degree: HashMap<ClassId, usize>,
}

impl Adjacency {
    /// Build adjacency from a diagram's edges.
    pub fn from_diagram(diagram: &Diagram) -> Self {
        // Count edges between pairs
        let mut pair_counts: HashMap<(ClassId, ClassId), usize> = HashMap::new();

        for edge in &diagram.edges {
            // Normalize pair order for bidirectional counting
            let pair = if edge.from.0 <= edge.to.0 {
                (edge.from, edge.to)
            } else {
                (edge.to, edge.from)
            };
            *pair_counts.entry(pair).or_default() += 1;
        }

        // Build neighbor lists and degree map
        let mut neighbors: HashMap<ClassId, Vec<(ClassId, usize)>> = HashMap::new();
        let mut degree: HashMap<ClassId, usize> = HashMap::new();

        for ((a, b), count) in pair_counts {
            // Add bidirectional entries
            neighbors.entry(a).or_default().push((b, count));
            if a != b {
                neighbors.entry(b).or_default().push((a, count));
            }

            *degree.entry(a).or_default() += count;
            if a != b {
                *degree.entry(b).or_default() += count;
            }
        }

        // Sort neighbor lists by (neighbor order, then neighbor id) for determinism
        for (_, list) in neighbors.iter_mut() {
            list.sort_by_key(|(cid, _)| {
                diagram.classes.get(cid.0).map(|c| c.order).unwrap_or(usize::MAX)
            });
        }

        Self { neighbors, degree }
    }

    /// Get the neighbors of a class, or empty slice if none.
    pub fn get_neighbors(&self, cid: ClassId) -> &[(ClassId, usize)] {
        self.neighbors.get(&cid).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get total degree (edge count) for a class.
    pub fn get_degree(&self, cid: ClassId) -> usize {
        self.degree.get(&cid).copied().unwrap_or(0)
    }
}

/// Compute node weights (w_in, w_out) for all nodes in a specific group.
pub fn compute_node_weights(
    diagram: &Diagram,
    gid: GroupId,
    adjacency: &Adjacency,
) -> HashMap<ClassId, NodeWeights> {
    let group = &diagram.groups[gid.0];
    let children: std::collections::HashSet<ClassId> =
        group.children_classes.iter().copied().collect();

    let mut weights: HashMap<ClassId, NodeWeights> = HashMap::new();

    for &cid in &group.children_classes {
        let mut w = NodeWeights::default();

        for (neighbor, count) in adjacency.get_neighbors(cid) {
            if children.contains(neighbor) {
                w.w_in += count;
            } else {
                w.w_out += count;
            }
        }

        weights.insert(cid, w);
    }

    weights
}

/// Compute inter-group edge counts for group-level adjacency.
/// Returns map from (group_a, group_b) -> edge_count (normalized so a.0 <= b.0).
pub fn compute_group_adjacency(diagram: &Diagram) -> HashMap<(GroupId, GroupId), usize> {
    let mut counts: HashMap<(GroupId, GroupId), usize> = HashMap::new();

    for edge in &diagram.edges {
        let from_group = diagram.classes[edge.from.0].group;
        let to_group = diagram.classes[edge.to.0].group;

        if from_group != to_group {
            let pair = if from_group.0 <= to_group.0 {
                (from_group, to_group)
            } else {
                (to_group, from_group)
            };
            *counts.entry(pair).or_default() += 1;
        }
    }

    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Ident, Group, Class, Edge, Arrow};

    fn make_test_diagram() -> Diagram {
        // A simple diagram: A -> B -> C, all in same group
        Diagram {
            root: GroupId(0),
            groups: vec![Group {
                gid: GroupId(0),
                id: None,
                parent: None,
                pos: None,
                children_groups: vec![],
                children_classes: vec![ClassId(0), ClassId(1), ClassId(2)],
                order: 0,
            }],
            classes: vec![
                Class {
                    cid: ClassId(0),
                    id: Ident("A".to_string()),
                    label: None,
                    group: GroupId(0),
                    pos: None,
                    body_lines: vec![],
                    order: 0,
                },
                Class {
                    cid: ClassId(1),
                    id: Ident("B".to_string()),
                    label: None,
                    group: GroupId(0),
                    pos: None,
                    body_lines: vec![],
                    order: 1,
                },
                Class {
                    cid: ClassId(2),
                    id: Ident("C".to_string()),
                    label: None,
                    group: GroupId(0),
                    pos: None,
                    body_lines: vec![],
                    order: 2,
                },
            ],
            edges: vec![
                Edge { from: ClassId(0), to: ClassId(1), arrow: Arrow::Line, label: None, order: 3 },
                Edge { from: ClassId(1), to: ClassId(2), arrow: Arrow::Line, label: None, order: 4 },
            ],
        }
    }

    #[test]
    fn test_adjacency_from_diagram() {
        let diagram = make_test_diagram();
        let adj = Adjacency::from_diagram(&diagram);

        // A has 1 neighbor (B)
        assert_eq!(adj.get_degree(ClassId(0)), 1);
        // B has 2 neighbors (A and C)
        assert_eq!(adj.get_degree(ClassId(1)), 2);
        // C has 1 neighbor (B)
        assert_eq!(adj.get_degree(ClassId(2)), 1);
    }

    #[test]
    fn test_node_weights_all_internal() {
        let diagram = make_test_diagram();
        let adj = Adjacency::from_diagram(&diagram);
        let weights = compute_node_weights(&diagram, GroupId(0), &adj);

        // All edges are internal
        for cid in [ClassId(0), ClassId(1), ClassId(2)] {
            let w = weights.get(&cid).unwrap();
            assert_eq!(w.w_out, 0);
        }
    }
}
