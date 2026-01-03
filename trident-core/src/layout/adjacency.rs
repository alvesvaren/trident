// Adjacency and edge weight computation for graph-driven layout.
//
// Builds data structures that describe node connectivity, used to:
// 1. Choose placement order (most-connected-to-placed first)
// 2. Generate candidate positions (near connected nodes)
// 3. Score candidates (minimize edge lengths)
// 4. Bias placement (external-facing nodes near boundaries)

use std::collections::HashMap;
use crate::parser::{Diagram, GroupId, NodeId};

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
    #[allow(dead_code)]
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
    /// For each node, list of (neighbor_node, edge_count).
    /// Edges are counted bidirectionally (both from→to and to→from).
    pub neighbors: HashMap<NodeId, Vec<(NodeId, usize)>>,

    /// Total degree (edge count) per node.
    pub degree: HashMap<NodeId, usize>,
}

impl Adjacency {
    /// Build adjacency from a diagram's edges.
    pub fn from_diagram(diagram: &Diagram) -> Self {
        // Count edges between pairs
        let mut pair_counts: HashMap<(NodeId, NodeId), usize> = HashMap::new();

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
        let mut neighbors: HashMap<NodeId, Vec<(NodeId, usize)>> = HashMap::new();
        let mut degree: HashMap<NodeId, usize> = HashMap::new();

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
            list.sort_by_key(|(nid, _)| {
                diagram.nodes.get(nid.0).map(|n| n.order).unwrap_or(usize::MAX)
            });
        }

        Self { neighbors, degree }
    }

    /// Get the neighbors of a node, or empty slice if none.
    pub fn get_neighbors(&self, nid: NodeId) -> &[(NodeId, usize)] {
        self.neighbors.get(&nid).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get total degree (edge count) for a node.
    pub fn get_degree(&self, nid: NodeId) -> usize {
        self.degree.get(&nid).copied().unwrap_or(0)
    }
}

/// Compute node weights (w_in, w_out) for all nodes in a specific group.
pub fn compute_node_weights(
    diagram: &Diagram,
    gid: GroupId,
    adjacency: &Adjacency,
) -> HashMap<NodeId, NodeWeights> {
    let group = &diagram.groups[gid.0];
    let children: std::collections::HashSet<NodeId> =
        group.children_nodes.iter().copied().collect();

    let mut weights: HashMap<NodeId, NodeWeights> = HashMap::new();

    for &nid in &group.children_nodes {
        let mut w = NodeWeights::default();

        for (neighbor, count) in adjacency.get_neighbors(nid) {
            if children.contains(neighbor) {
                w.w_in += count;
            } else {
                w.w_out += count;
            }
        }

        weights.insert(nid, w);
    }

    weights
}

/// Compute inter-group edge counts for group-level adjacency.
/// Returns map from (group_a, group_b) -> edge_count (normalized so a.0 <= b.0).
#[allow(dead_code)]
pub fn compute_group_adjacency(diagram: &Diagram) -> HashMap<(GroupId, GroupId), usize> {
    let mut counts: HashMap<(GroupId, GroupId), usize> = HashMap::new();

    for edge in &diagram.edges {
        let from_group = diagram.nodes[edge.from.0].group;
        let to_group = diagram.nodes[edge.to.0].group;

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
    use crate::parser::{Ident, Group, Node, Edge};

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
                children_nodes: vec![NodeId(0), NodeId(1), NodeId(2)],
                order: 0,
            }],
            nodes: vec![
                Node {
                    nid: NodeId(0),
                    kind: "class".to_string(),
                    modifiers: vec![],
                    id: Ident("A".to_string()),
                    label: None,
                    group: GroupId(0),
                    pos: None,
                    width: None,
                    height: None,
                    body_lines: vec![],
                    explicit: true,
                    order: 0,
                },
                Node {
                    nid: NodeId(1),
                    kind: "class".to_string(),
                    modifiers: vec![],
                    id: Ident("B".to_string()),
                    label: None,
                    group: GroupId(0),
                    pos: None,
                    width: None,
                    height: None,
                    body_lines: vec![],
                    explicit: true,
                    order: 1,
                },
                Node {
                    nid: NodeId(2),
                    kind: "class".to_string(),
                    modifiers: vec![],
                    id: Ident("C".to_string()),
                    label: None,
                    group: GroupId(0),
                    pos: None,
                    width: None,
                    height: None,
                    body_lines: vec![],
                    explicit: true,
                    order: 2,
                },
            ],
            edges: vec![
                Edge { from: NodeId(0), to: NodeId(1), arrow: "line".to_string(), label: None, order: 3 },
                Edge { from: NodeId(1), to: NodeId(2), arrow: "line".to_string(), label: None, order: 4 },
            ],
        }
    }

    #[test]
    fn test_adjacency_from_diagram() {
        let diagram = make_test_diagram();
        let adj = Adjacency::from_diagram(&diagram);

        // A has 1 neighbor (B)
        assert_eq!(adj.get_degree(NodeId(0)), 1);
        // B has 2 neighbors (A and C)
        assert_eq!(adj.get_degree(NodeId(1)), 2);
        // C has 1 neighbor (B)
        assert_eq!(adj.get_degree(NodeId(2)), 1);
    }

    #[test]
    fn test_node_weights_all_internal() {
        let diagram = make_test_diagram();
        let adj = Adjacency::from_diagram(&diagram);
        let weights = compute_node_weights(&diagram, GroupId(0), &adj);

        // All edges are internal
        for nid in [NodeId(0), NodeId(1), NodeId(2)] {
            let w = weights.get(&nid).unwrap();
            assert_eq!(w.w_out, 0);
        }
    }
}
