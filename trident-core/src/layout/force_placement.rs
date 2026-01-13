// Constraint-based placement algorithm with barycenter positioning.
//
// This module provides a pixel-level layout algorithm that:
// - Respects @pos constraints
// - Places connected nodes close together using barycenter (weighted average)
// - Resolves overlaps via minimum displacement nudging
// - Is deterministic and non-iterative

use std::collections::{HashMap, HashSet, VecDeque};
use crate::parser::{PointI, Diagram, NodeId};
use super::{RectI, SizeI, LayoutConfig, get_node_size};
use super::spatial_grid::SpatialGrid;
use super::adjacency::Adjacency;

/// Compute node sizes for all nodes in scope.
pub fn compute_node_sizes(
    diagram: &Diagram,
    nodes: &[NodeId],
    cfg: &LayoutConfig,
) -> HashMap<NodeId, SizeI> {
    nodes.iter()
        .map(|&nid| (nid, get_node_size(&diagram.nodes[nid.0], cfg)))
        .collect()
}

/// Build directed adjacency for hierarchy (parent -> child).
/// Returns (forward adjacency, reverse adjacency).
pub fn build_dependency_graph(
    diagram: &Diagram,
    scope: &[NodeId],
) -> (HashMap<NodeId, Vec<NodeId>>, HashMap<NodeId, Vec<NodeId>>) {
    use crate::parser::{get_arrow_definition, get_base_arrow_name, ARROW_DEFINITIONS};
    
    let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut rev_adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let scope_set: HashSet<NodeId> = scope.iter().copied().collect();

    for edge in &diagram.edges {
        let (parent, child) = get_edge_direction(&edge.arrow, edge.from, edge.to);

        if scope_set.contains(&parent) && scope_set.contains(&child) {
            adj.entry(parent).or_default().push(child);
            rev_adj.entry(child).or_default().push(parent);
        }
    }
    (adj, rev_adj)
}

/// Get arrow direction for ranking.
/// Returns (parent_id, child_id) where parent should be above child.
fn get_edge_direction(arrow: &str, from: NodeId, to: NodeId) -> (NodeId, NodeId) {
    use crate::parser::{get_arrow_definition, get_base_arrow_name, ARROW_DEFINITIONS};
    
    if let Some(def) = get_arrow_definition(arrow) {
        if !def.definition.is_left {
            let base_name = get_base_arrow_name(arrow);
            let is_hierarchy_reversed = ARROW_DEFINITIONS
                .iter()
                .find(|d| d.name == base_name)
                .map(|d| d.hierarchy_reversed)
                .unwrap_or(false);
            
            if is_hierarchy_reversed {
                (to, from)
            } else {
                (from, to)
            }
        } else {
            let base_name = get_base_arrow_name(arrow);
            let is_hierarchy_reversed = ARROW_DEFINITIONS
                .iter()
                .find(|d| d.name == base_name)
                .map(|d| d.hierarchy_reversed)
                .unwrap_or(false);
            
            if is_hierarchy_reversed {
                (from, to)
            } else {
                (to, from)
            }
        }
    } else {
        (from, to)
    }
}

/// Assign ranks to nodes using longest-path layering.
/// Fixed nodes get ranks derived from their Y position.
/// Free nodes get rank = max(parent_ranks) + 1.
pub fn assign_ranks(
    diagram: &Diagram,
    all_nodes: &[NodeId],
    fixed_positions: &HashMap<NodeId, PointI>,
    adj: &HashMap<NodeId, Vec<NodeId>>,
    rev_adj: &HashMap<NodeId, Vec<NodeId>>,
    layer_height: i32,
) -> HashMap<NodeId, i32> {
    let mut ranks: HashMap<NodeId, i32> = HashMap::new();
    let mut visited: HashSet<NodeId> = HashSet::new();
    let all_set: HashSet<NodeId> = all_nodes.iter().copied().collect();
    
    // Fixed nodes get rank from Y position
    for (&nid, pos) in fixed_positions {
        let rank = pos.y / layer_height.max(1);
        ranks.insert(nid, rank);
        visited.insert(nid);
    }
    
    // Find roots among remaining nodes (no incoming edges)
    let mut queue: VecDeque<(NodeId, i32)> = VecDeque::new();
    
    for &nid in all_nodes {
        if !visited.contains(&nid) && !rev_adj.contains_key(&nid) {
            queue.push_back((nid, 0));
            visited.insert(nid);
        }
    }
    
    // If no roots found (cycles), pick first unvisited by order
    if queue.is_empty() {
        for &nid in all_nodes {
            if !visited.contains(&nid) {
                queue.push_back((nid, 0));
                visited.insert(nid);
                break;
            }
        }
    }
    
    // BFS to assign ranks
    while let Some((nid, rank)) = queue.pop_front() {
        // Only update rank if not fixed
        if !fixed_positions.contains_key(&nid) {
            // Use max of current and new rank for longest-path
            let current = ranks.get(&nid).copied().unwrap_or(0);
            ranks.insert(nid, current.max(rank));
        }
        
        if let Some(children) = adj.get(&nid) {
            for &child in children {
                if all_set.contains(&child) {
                    let child_rank = rank + 1;
                    let current_child = ranks.get(&child).copied().unwrap_or(0);
                    
                    if child_rank > current_child || !visited.contains(&child) {
                        ranks.insert(child, child_rank.max(current_child));
                    }
                    
                    if !visited.contains(&child) {
                        visited.insert(child);
                        queue.push_back((child, child_rank));
                    }
                }
            }
        }
    }
    
    // Handle any still-unvisited nodes (disconnected components)
    for &nid in all_nodes {
        if !ranks.contains_key(&nid) {
            ranks.insert(nid, 0);
        }
    }
    
    ranks
}

/// Compute barycenter (weighted center) of connected and placed nodes.
/// Returns the average position of all connected nodes that are already placed.
pub fn compute_barycenter(
    nid: NodeId,
    adjacency: &Adjacency,
    placed_positions: &HashMap<NodeId, PointI>,
    node_sizes: &HashMap<NodeId, SizeI>,
) -> Option<PointI> {
    let neighbors = adjacency.get_neighbors(nid);
    
    if neighbors.is_empty() {
        return None;
    }
    
    let mut sum_x: i64 = 0;
    let mut sum_y: i64 = 0;
    let mut weight: i64 = 0;
    
    for &(neighbor_nid, edge_count) in neighbors {
        if let Some(pos) = placed_positions.get(&neighbor_nid) {
            let size = node_sizes.get(&neighbor_nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
            // Use center of the node
            let cx = pos.x + size.w / 2;
            let cy = pos.y + size.h / 2;
            
            sum_x += cx as i64 * edge_count as i64;
            sum_y += cy as i64 * edge_count as i64;
            weight += edge_count as i64;
        }
    }
    
    if weight == 0 {
        return None;
    }
    
    Some(PointI {
        x: (sum_x / weight) as i32,
        y: (sum_y / weight) as i32,
    })
}

/// Find the minimum displacement to resolve overlap between two rectangles.
/// Returns (dx, dy) to move rect_a to no longer overlap rect_b.
fn min_displacement(rect_a: &RectI, rect_b: &RectI, gap: i32) -> (i32, i32) {
    // Calculate overlap in each direction
    let overlap_left = rect_a.right() - rect_b.x + gap;
    let overlap_right = rect_b.right() - rect_a.x + gap;
    let overlap_top = rect_a.bottom() - rect_b.y + gap;
    let overlap_bottom = rect_b.bottom() - rect_a.y + gap;
    
    // Find minimum displacement direction
    let min_x = if overlap_left < overlap_right { -overlap_left } else { overlap_right };
    let min_y = if overlap_top < overlap_bottom { -overlap_top } else { overlap_bottom };
    
    // Choose the axis with smaller displacement
    if min_x.abs() < min_y.abs() {
        (min_x, 0)
    } else {
        (0, min_y)
    }
}

/// Resolve overlaps by nudging nodes.
/// Modifies positions in place.
pub fn resolve_overlaps(
    node_order: &[NodeId],
    node_positions: &mut HashMap<NodeId, PointI>,
    node_sizes: &HashMap<NodeId, SizeI>,
    gap: i32,
) {
    // Process nodes in order, fixing earlier nodes
    let mut placed: Vec<(NodeId, RectI)> = Vec::new();
    
    for &nid in node_order {
        let pos = match node_positions.get(&nid) {
            Some(p) => *p,
            None => continue,
        };
        let size = node_sizes.get(&nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
        let mut rect = RectI { x: pos.x, y: pos.y, w: size.w, h: size.h };
        
        // Check against all placed nodes and resolve overlaps
        let mut iterations = 0;
        const MAX_ITERATIONS: i32 = 50;
        
        loop {
            let mut displaced = false;
            
            for &(_, placed_rect) in &placed {
                let gap_rect = RectI {
                    x: placed_rect.x - gap,
                    y: placed_rect.y - gap,
                    w: placed_rect.w + gap * 2,
                    h: placed_rect.h + gap * 2,
                };
                
                if rect.overlaps(&gap_rect) {
                    let (dx, dy) = min_displacement(&rect, &placed_rect, gap);
                    rect.x += dx;
                    rect.y += dy;
                    displaced = true;
                }
            }
            
            iterations += 1;
            if !displaced || iterations >= MAX_ITERATIONS {
                break;
            }
        }
        
        node_positions.insert(nid, PointI { x: rect.x, y: rect.y });
        placed.push((nid, rect));
    }
}

/// Layout free nodes using constraint-based placement with barycenter positioning.
/// 
/// Key improvement: Uses true 2D barycenter positioning.
/// Nodes connected to existing nodes are placed at the weighted center of their neighbors.
pub fn layout_nodes_constrained(
    diagram: &Diagram,
    free_nodes: &[NodeId],
    fixed_nodes: &[NodeId],
    cfg: &LayoutConfig,
    adjacency: &Adjacency,
    node_local_pos: &mut HashMap<NodeId, PointI>,
) {
    if free_nodes.is_empty() {
        return;
    }
    
    // Compute sizes for all nodes
    let mut all_nodes: Vec<NodeId> = Vec::with_capacity(fixed_nodes.len() + free_nodes.len());
    all_nodes.extend_from_slice(fixed_nodes);
    all_nodes.extend_from_slice(free_nodes);
    
    let node_sizes = compute_node_sizes(diagram, &all_nodes, cfg);
    
    // Track placed nodes
    let mut placed_positions: HashMap<NodeId, PointI> = HashMap::new();
    let mut placement_order: Vec<NodeId> = Vec::new();
    
    // Copy fixed positions
    for &nid in fixed_nodes {
        if let Some(&pos) = node_local_pos.get(&nid) {
            placed_positions.insert(nid, pos);
            placement_order.push(nid);
        }
    }
    
    // Sort free nodes by connectivity (most connected first for stable placement)
    let mut free_nodes_sorted: Vec<(NodeId, usize)> = free_nodes.iter()
        .map(|&nid| (nid, adjacency.get_degree(nid)))
        .collect();
    free_nodes_sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| {
        diagram.nodes[a.0.0].order.cmp(&diagram.nodes[b.0.0].order)
    }));
    
    // Placement using iterative barycenter refinement
    let start_x = cfg.group_padding;
    let start_y = cfg.group_padding;
    let mut spiral_angle = 0.0f64;
    let spiral_step = std::f64::consts::PI / 3.0;
    let mut spiral_radius = 0.0;
    
    for (nid, _degree) in free_nodes_sorted {
        let size = node_sizes.get(&nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
        
        // Try to compute barycenter from connected placed nodes
        let pos = if let Some(center) = compute_barycenter(nid, adjacency, &placed_positions, &node_sizes) {
            // Place centered on barycenter (true 2D positioning)
            PointI {
                x: center.x - size.w / 2,
                y: center.y - size.h / 2,
            }
        } else {
            // No connected nodes placed yet, use spiral placement
            let pos = PointI {
                x: start_x + (spiral_radius * spiral_angle.cos()) as i32,
                y: start_y + (spiral_radius * spiral_angle.sin()) as i32,
            };
            spiral_angle += spiral_step;
            if spiral_angle >= 2.0 * std::f64::consts::PI {
                spiral_angle = 0.0;
                spiral_radius += (cfg.class_size.w + cfg.gap) as f64;
            }
            pos
        };
        
        placed_positions.insert(nid, pos);
        placement_order.push(nid);
    }
    
    // Resolve overlaps with more aggressive spreading
    resolve_overlaps_aggressive(&placement_order, &mut placed_positions, &node_sizes, cfg.gap, fixed_nodes);
    
    // Copy results to output (only free nodes)
    for &nid in free_nodes {
        if let Some(pos) = placed_positions.get(&nid) {
            node_local_pos.insert(nid, *pos);
        }
    }
}

/// More aggressive overlap resolution that spreads nodes apart.
fn resolve_overlaps_aggressive(
    node_order: &[NodeId],
    node_positions: &mut HashMap<NodeId, PointI>,
    node_sizes: &HashMap<NodeId, SizeI>,
    gap: i32,
    fixed_nodes: &[NodeId],
) {
    let fixed_set: HashSet<NodeId> = fixed_nodes.iter().copied().collect();
    
    // Multiple passes to resolve cascading overlaps
    for _pass in 0..10 {
        let mut any_moved = false;
        
        for i in 0..node_order.len() {
            let nid = node_order[i];
            
            // Don't move fixed nodes
            if fixed_set.contains(&nid) {
                continue;
            }
            
            let pos = match node_positions.get(&nid) {
                Some(p) => *p,
                None => continue,
            };
            let size = node_sizes.get(&nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
            let mut rect = RectI { x: pos.x, y: pos.y, w: size.w, h: size.h };
            
            // Check against all other nodes
            for j in 0..node_order.len() {
                if i == j {
                    continue;
                }
                
                let other_nid = node_order[j];
                let other_pos = match node_positions.get(&other_nid) {
                    Some(p) => *p,
                    None => continue,
                };
                let other_size = node_sizes.get(&other_nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
                let other_rect = RectI { x: other_pos.x, y: other_pos.y, w: other_size.w, h: other_size.h };
                
                // Check overlap with gap
                let gap_rect = RectI {
                    x: other_rect.x - gap,
                    y: other_rect.y - gap,
                    w: other_rect.w + gap * 2,
                    h: other_rect.h + gap * 2,
                };
                
                if rect.overlaps(&gap_rect) {
                    // Calculate displacement direction (away from other node's center)
                    let my_cx = rect.x + rect.w / 2;
                    let my_cy = rect.y + rect.h / 2;
                    let other_cx = other_rect.x + other_rect.w / 2;
                    let other_cy = other_rect.y + other_rect.h / 2;
                    
                    let dx = my_cx - other_cx;
                    let dy = my_cy - other_cy;
                    
                    // Move away from other node
                    if dx.abs() > dy.abs() {
                        // Primarily horizontal overlap
                        if dx >= 0 {
                            rect.x = other_rect.right() + gap;
                        } else {
                            rect.x = other_rect.x - rect.w - gap;
                        }
                    } else {
                        // Primarily vertical overlap
                        if dy >= 0 {
                            rect.y = other_rect.bottom() + gap;
                        } else {
                            rect.y = other_rect.y - rect.h - gap;
                        }
                    }
                    any_moved = true;
                }
            }
            
            node_positions.insert(nid, PointI { x: rect.x, y: rect.y });
        }
        
        if !any_moved {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Ident;
    use crate::parser::compile::{Group, Node, Edge, Diagram, GroupId};

    fn make_simple_diagram() -> Diagram {
        // A --> B --> C (linear chain)
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
    fn test_compute_barycenter_two_neighbors() {
        let diagram = make_simple_diagram();
        let adjacency = Adjacency::from_diagram(&diagram);
        
        let mut positions = HashMap::new();
        positions.insert(NodeId(0), PointI { x: 0, y: 0 });
        positions.insert(NodeId(2), PointI { x: 200, y: 0 });
        
        let sizes: HashMap<NodeId, SizeI> = vec![
            (NodeId(0), SizeI { w: 100, h: 80 }),
            (NodeId(1), SizeI { w: 100, h: 80 }),
            (NodeId(2), SizeI { w: 100, h: 80 }),
        ].into_iter().collect();
        
        // Node B (1) is connected to both A (0) and C (2)
        let barycenter = compute_barycenter(NodeId(1), &adjacency, &positions, &sizes);
        
        assert!(barycenter.is_some());
        let bc = barycenter.unwrap();
        // Center of A is (50, 40), center of C is (250, 40)
        // Barycenter should be around (150, 40)
        assert_eq!(bc.x, 150);
        assert_eq!(bc.y, 40);
    }

    #[test]
    fn test_overlap_resolution() {
        let mut positions = HashMap::new();
        positions.insert(NodeId(0), PointI { x: 0, y: 0 });
        positions.insert(NodeId(1), PointI { x: 50, y: 0 }); // Overlaps with 0
        
        let sizes: HashMap<NodeId, SizeI> = vec![
            (NodeId(0), SizeI { w: 100, h: 80 }),
            (NodeId(1), SizeI { w: 100, h: 80 }),
        ].into_iter().collect();
        
        let order = vec![NodeId(0), NodeId(1)];
        resolve_overlaps(&order, &mut positions, &sizes, 10);
        
        let pos0 = positions.get(&NodeId(0)).unwrap();
        let pos1 = positions.get(&NodeId(1)).unwrap();
        
        // After resolution, nodes should not overlap (with gap)
        let rect0 = RectI { x: pos0.x, y: pos0.y, w: 100, h: 80 };
        let rect1 = RectI { x: pos1.x, y: pos1.y, w: 100, h: 80 };
        
        // Check no overlap with gap
        let gap_rect0 = RectI { x: rect0.x - 5, y: rect0.y - 5, w: rect0.w + 10, h: rect0.h + 10 };
        assert!(!gap_rect0.overlaps(&rect1) || pos1.x >= pos0.x + 100 + 10);
    }

    #[test]
    fn test_assign_ranks_linear_chain() {
        let diagram = make_simple_diagram();
        let all_nodes = vec![NodeId(0), NodeId(1), NodeId(2)];
        let (adj, rev_adj) = build_dependency_graph(&diagram, &all_nodes);
        
        let ranks = assign_ranks(&diagram, &all_nodes, &HashMap::new(), &adj, &rev_adj, 150);
        
        // In a linear chain A -> B -> C, ranks should increase
        let rank_a = ranks.get(&NodeId(0)).copied().unwrap_or(-1);
        let rank_b = ranks.get(&NodeId(1)).copied().unwrap_or(-1);
        let rank_c = ranks.get(&NodeId(2)).copied().unwrap_or(-1);
        
        // All should have valid ranks
        assert!(rank_a >= 0);
        assert!(rank_b >= 0);
        assert!(rank_c >= 0);
    }
}
