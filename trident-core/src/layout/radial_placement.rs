// Radial Tree Layout Algorithm ("Orbital" Layout)
//
// This module implements a radial/spiral tree layout that:
// 1. Finds the most connected node as "root"
// 2. Builds a spanning tree (breaking cycles at weakest links)
// 3. Places nodes in circular arcs around their parents
//
// Properties:
// - Deterministic (no randomness)
// - Pixel-level precision
// - Mind-map style visualization
// - Respects @pos constraints

use std::collections::{HashMap, HashSet, BinaryHeap};
use std::cmp::Ordering;
use crate::parser::{PointI, Diagram, NodeId};
use super::{SizeI, LayoutConfig, get_node_size};
use super::adjacency::Adjacency;

/// Edge for spanning tree construction with weight.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct WeightedEdge {
    weight: i32,  // Higher = stronger connection (keep in tree)
    from: NodeId,
    to: NodeId,
}

impl Ord for WeightedEdge {
    fn cmp(&self, other: &Self) -> Ordering {
        // Max-heap: higher weight first
        self.weight.cmp(&other.weight)
            .then_with(|| self.from.0.cmp(&other.from.0))
            .then_with(|| self.to.0.cmp(&other.to.0))
    }
}

impl PartialOrd for WeightedEdge {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Tree node for layout computation.
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub nid: NodeId,
    pub children: Vec<NodeId>,
    pub depth: i32,
    /// Angle offset for this node's subtree (in radians)
    pub angle_start: f64,
    /// Angular width of this node's subtree
    pub angle_span: f64,
}

/// Result of tree building.
#[derive(Debug)]
pub struct LayoutTree {
    pub root: NodeId,
    pub nodes: HashMap<NodeId, TreeNode>,
    pub parent: HashMap<NodeId, NodeId>,
}

/// Compute edge weight based on arrow type.
/// Higher weight = stronger connection (should be in tree).
fn get_edge_weight(arrow: &str) -> i32 {
    use crate::parser::{get_arrow_definition, get_base_arrow_name, ARROW_DEFINITIONS};
    
    let base = get_base_arrow_name(arrow);
    
    // Hierarchy arrows are strongest (extends, implements)
    if base == "extends" || base == "implements" {
        return 100;
    }
    
    // Composition/aggregation are strong
    if base == "composition" || base == "aggregation" {
        return 80;
    }
    
    // Dependencies are medium
    if base == "dependency" || base == "realizes" {
        return 60;
    }
    
    // Associations are moderate
    if base == "association" || base == "assoc" {
        return 40;
    }
    
    // Default: simple line
    20
}

/// Find the most connected node to use as root.
/// Uses deterministic ordering: first by degree, then by source order.
fn find_root(
    diagram: &Diagram,
    nodes: &[NodeId],
    adjacency: &Adjacency,
    fixed_positions: &HashMap<NodeId, PointI>,
) -> NodeId {
    // If there are fixed nodes, pick the one with lowest source order
    if !fixed_positions.is_empty() {
        return *fixed_positions.keys()
            .min_by_key(|&&nid| diagram.nodes[nid.0].order)
            .unwrap();
    }
    
    // Otherwise, pick highest degree node (tie-break by source order)
    nodes.iter()
        .max_by_key(|&&nid| {
            let degree = adjacency.get_degree(nid);
            let order = diagram.nodes[nid.0].order;
            // High degree first, then low order
            (degree, std::cmp::Reverse(order))
        })
        .copied()
        .unwrap_or(nodes[0])
}

/// Build a maximum spanning tree using Prim's algorithm.
/// Returns the tree structure.
pub fn build_spanning_tree(
    diagram: &Diagram,
    nodes: &[NodeId],
    adjacency: &Adjacency,
    fixed_positions: &HashMap<NodeId, PointI>,
) -> LayoutTree {
    let node_set: HashSet<NodeId> = nodes.iter().copied().collect();
    
    if nodes.is_empty() {
        return LayoutTree {
            root: NodeId(0),
            nodes: HashMap::new(),
            parent: HashMap::new(),
        };
    }
    
    let root = find_root(diagram, nodes, adjacency, fixed_positions);
    
    // Prim's algorithm for maximum spanning tree
    let mut in_tree: HashSet<NodeId> = HashSet::new();
    let mut tree_nodes: HashMap<NodeId, TreeNode> = HashMap::new();
    let mut parent_map: HashMap<NodeId, NodeId> = HashMap::new();
    let mut heap: BinaryHeap<WeightedEdge> = BinaryHeap::new();
    
    // Start with root
    in_tree.insert(root);
    tree_nodes.insert(root, TreeNode {
        nid: root,
        children: Vec::new(),
        depth: 0,
        angle_start: 0.0,
        angle_span: 2.0 * std::f64::consts::PI,
    });
    
    // Add root's edges to heap
    for edge in &diagram.edges {
        if edge.from == root && node_set.contains(&edge.to) {
            heap.push(WeightedEdge {
                weight: get_edge_weight(&edge.arrow),
                from: root,
                to: edge.to,
            });
        }
        if edge.to == root && node_set.contains(&edge.from) {
            heap.push(WeightedEdge {
                weight: get_edge_weight(&edge.arrow),
                from: root,
                to: edge.from,
            });
        }
    }
    
    // Build tree
    while let Some(edge) = heap.pop() {
        if in_tree.contains(&edge.to) {
            continue;
        }
        
        // Add to tree
        in_tree.insert(edge.to);
        parent_map.insert(edge.to, edge.from);
        
        let parent_depth = tree_nodes.get(&edge.from).map(|n| n.depth).unwrap_or(0);
        tree_nodes.insert(edge.to, TreeNode {
            nid: edge.to,
            children: Vec::new(),
            depth: parent_depth + 1,
            angle_start: 0.0,
            angle_span: 0.0,
        });
        
        // Update parent's children (keep sorted by source order for determinism)
        if let Some(parent) = tree_nodes.get_mut(&edge.from) {
            parent.children.push(edge.to);
            parent.children.sort_by_key(|&nid| diagram.nodes[nid.0].order);
        }
        
        // Add new node's edges
        for diagram_edge in &diagram.edges {
            let neighbor = if diagram_edge.from == edge.to {
                diagram_edge.to
            } else if diagram_edge.to == edge.to {
                diagram_edge.from
            } else {
                continue;
            };
            
            if !in_tree.contains(&neighbor) && node_set.contains(&neighbor) {
                heap.push(WeightedEdge {
                    weight: get_edge_weight(&diagram_edge.arrow),
                    from: edge.to,
                    to: neighbor,
                });
            }
        }
    }
    
    // Handle disconnected nodes (add them as children of root)
    for &nid in nodes {
        if !in_tree.contains(&nid) {
            in_tree.insert(nid);
            parent_map.insert(nid, root);
            tree_nodes.insert(nid, TreeNode {
                nid,
                children: Vec::new(),
                depth: 1,
                angle_start: 0.0,
                angle_span: 0.0,
            });
            if let Some(root_node) = tree_nodes.get_mut(&root) {
                root_node.children.push(nid);
                root_node.children.sort_by_key(|&n| diagram.nodes[n.0].order);
            }
        }
    }
    
    LayoutTree {
        root,
        nodes: tree_nodes,
        parent: parent_map,
    }
}

/// Count total descendants (including self) for subtree sizing.
fn count_descendants(tree: &LayoutTree, nid: NodeId) -> usize {
    let mut count = 1; // self
    if let Some(node) = tree.nodes.get(&nid) {
        for &child in &node.children {
            count += count_descendants(tree, child);
        }
    }
    count
}

/// Assign angular spans to each node based on subtree size.
fn assign_angles(tree: &mut LayoutTree, nid: NodeId, start: f64, span: f64) {
    if let Some(node) = tree.nodes.get_mut(&nid) {
        node.angle_start = start;
        node.angle_span = span;
    }
    
    let children: Vec<NodeId> = tree.nodes.get(&nid)
        .map(|n| n.children.clone())
        .unwrap_or_default();
    
    if children.is_empty() {
        return;
    }
    
    // Count total descendants for each child
    let child_weights: Vec<(NodeId, usize)> = children.iter()
        .map(|&c| (c, count_descendants(tree, c)))
        .collect();
    
    let total_weight: usize = child_weights.iter().map(|(_, w)| w).sum();
    
    // Distribute angles proportionally
    let mut current_angle = start;
    for (child, weight) in child_weights {
        let child_span = if total_weight > 0 {
            span * (weight as f64 / total_weight as f64)
        } else {
            span / children.len() as f64
        };
        
        assign_angles(tree, child, current_angle, child_span);
        current_angle += child_span;
    }
}

/// Compute radial positions for all nodes.
pub fn compute_radial_positions(
    tree: &LayoutTree,
    node_sizes: &HashMap<NodeId, SizeI>,
    cfg: &LayoutConfig,
    fixed_positions: &HashMap<NodeId, PointI>,
) -> HashMap<NodeId, PointI> {
    let mut positions: HashMap<NodeId, PointI> = HashMap::new();
    
    // Calculate radius per depth level - COMPACT version
    // Average node size + minimal gap
    let avg_size = node_sizes.values()
        .map(|s| (s.w + s.h) / 2)
        .sum::<i32>() / node_sizes.len().max(1) as i32;
    let level_radius = (avg_size + cfg.gap) as f64;
    
    // Start from center
    let center_x = 400; // Will be adjusted later
    let center_y = 400;
    
    // Place root at center (or use fixed position)
    let root_pos = fixed_positions.get(&tree.root)
        .copied()
        .unwrap_or_else(|| {
            let size = node_sizes.get(&tree.root).copied().unwrap_or(SizeI { w: 100, h: 80 });
            PointI { x: center_x - size.w / 2, y: center_y - size.h / 2 }
        });
    positions.insert(tree.root, root_pos);
    
    // BFS to place children
    let mut queue: Vec<NodeId> = vec![tree.root];
    let mut visited: HashSet<NodeId> = HashSet::new();
    visited.insert(tree.root);
    
    while let Some(parent_nid) = queue.pop() {
        let parent_pos = positions.get(&parent_nid).copied().unwrap_or(root_pos);
        let parent_size = node_sizes.get(&parent_nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
        let parent_cx = parent_pos.x + parent_size.w / 2;
        let parent_cy = parent_pos.y + parent_size.h / 2;
        
        if let Some(parent_node) = tree.nodes.get(&parent_nid) {
            for &child_nid in &parent_node.children {
                if visited.contains(&child_nid) {
                    continue;
                }
                visited.insert(child_nid);
                
                // Check for fixed position
                if let Some(&fixed) = fixed_positions.get(&child_nid) {
                    positions.insert(child_nid, fixed);
                    queue.push(child_nid);
                    continue;
                }
                
                if let Some(child_node) = tree.nodes.get(&child_nid) {
                    let child_size = node_sizes.get(&child_nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
                    
                    // Calculate position on arc
                    let angle = child_node.angle_start + child_node.angle_span / 2.0;
                    let radius = level_radius * (child_node.depth as f64);
                    
                    // Position relative to root (not parent) for cleaner radial layout
                    let root_cx = root_pos.x + node_sizes.get(&tree.root).map(|s| s.w/2).unwrap_or(50);
                    let root_cy = root_pos.y + node_sizes.get(&tree.root).map(|s| s.h/2).unwrap_or(40);
                    
                    let x = root_cx + (radius * angle.cos()) as i32 - child_size.w / 2;
                    let y = root_cy + (radius * angle.sin()) as i32 - child_size.h / 2;
                    
                    positions.insert(child_nid, PointI { x, y });
                    queue.push(child_nid);
                }
            }
        }
    }
    
    positions
}

/// Main entry point: layout nodes using radial tree algorithm.
pub fn layout_nodes_radial(
    diagram: &Diagram,
    free_nodes: &[NodeId],
    fixed_nodes: &[NodeId],
    cfg: &LayoutConfig,
    adjacency: &Adjacency,
    node_local_pos: &mut HashMap<NodeId, PointI>,
) {
    if free_nodes.is_empty() && fixed_nodes.is_empty() {
        return;
    }
    
    // Combine all nodes
    let mut all_nodes: Vec<NodeId> = Vec::with_capacity(free_nodes.len() + fixed_nodes.len());
    all_nodes.extend_from_slice(fixed_nodes);
    all_nodes.extend_from_slice(free_nodes);
    
    // Compute node sizes
    let node_sizes: HashMap<NodeId, SizeI> = all_nodes.iter()
        .map(|&nid| (nid, get_node_size(&diagram.nodes[nid.0], cfg)))
        .collect();
    
    // Get fixed positions
    let fixed_positions: HashMap<NodeId, PointI> = fixed_nodes.iter()
        .filter_map(|&nid| node_local_pos.get(&nid).map(|p| (nid, *p)))
        .collect();
    
    // Build spanning tree
    let mut tree = build_spanning_tree(diagram, &all_nodes, adjacency, &fixed_positions);
    
    // Assign angles starting from root
    let root_nid = tree.root;
    assign_angles(&mut tree, root_nid, 0.0, 2.0 * std::f64::consts::PI);
    
    // Compute positions
    let mut positions = compute_radial_positions(&tree, &node_sizes, cfg, &fixed_positions);
    
    // Light force-directed refinement: just enough to resolve overlaps and tighten edges
    // Few iterations to preserve initial radial structure
    const FORCE_ITERATIONS: usize = 10;
    for _iter in 0..FORCE_ITERATIONS {
        // Attraction: pull connected nodes together (gently)
        apply_attraction(&mut positions, &node_sizes, diagram, adjacency, &fixed_positions, cfg.gap);
        
        // Edge repulsion: push nodes away from edge paths (prevents arrows going "behind" nodes)
        apply_edge_repulsion(&mut positions, &node_sizes, diagram, &fixed_positions, cfg.gap);
        
        // Repulsion: push overlapping nodes apart
        resolve_radial_overlaps(&mut positions, &node_sizes, cfg.gap, &fixed_positions);
    }
    
    // Copy to output (only free nodes, keep fixed as-is)
    for &nid in free_nodes {
        if let Some(&pos) = positions.get(&nid) {
            node_local_pos.insert(nid, pos);
        }
    }
}

/// Push nodes away from edge paths to prevent arrows going "behind" nodes.
fn apply_edge_repulsion(
    positions: &mut HashMap<NodeId, PointI>,
    node_sizes: &HashMap<NodeId, SizeI>,
    diagram: &Diagram,
    fixed_positions: &HashMap<NodeId, PointI>,
    gap: i32,
) {
    let fixed_set: HashSet<NodeId> = fixed_positions.keys().copied().collect();
    
    // Sort nodes for determinism
    let mut node_list: Vec<NodeId> = positions.keys().copied().collect();
    node_list.sort_by_key(|nid| nid.0);
    
    let mut movements: HashMap<NodeId, (f64, f64)> = HashMap::new();
    
    for &nid in &node_list {
        if fixed_set.contains(&nid) {
            continue;
        }
        
        let pos = match positions.get(&nid) {
            Some(p) => *p,
            None => continue,
        };
        let size = node_sizes.get(&nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
        let node_cx = (pos.x + size.w / 2) as f64;
        let node_cy = (pos.y + size.h / 2) as f64;
        let node_radius = ((size.w + size.h) / 4 + gap) as f64;
        
        let mut total_dx = 0.0f64;
        let mut total_dy = 0.0f64;
        let mut push_count = 0;
        
        // Check each edge
        for edge in &diagram.edges {
            // Skip if this node is part of the edge
            if edge.from == nid || edge.to == nid {
                continue;
            }
            
            let from_pos = match positions.get(&edge.from) {
                Some(p) => *p,
                None => continue,
            };
            let to_pos = match positions.get(&edge.to) {
                Some(p) => *p,
                None => continue,
            };
            
            let from_size = node_sizes.get(&edge.from).copied().unwrap_or(SizeI { w: 100, h: 80 });
            let to_size = node_sizes.get(&edge.to).copied().unwrap_or(SizeI { w: 100, h: 80 });
            
            let ax = (from_pos.x + from_size.w / 2) as f64;
            let ay = (from_pos.y + from_size.h / 2) as f64;
            let bx = (to_pos.x + to_size.w / 2) as f64;
            let by = (to_pos.y + to_size.h / 2) as f64;
            
            // Calculate closest point on line segment to node center
            let abx = bx - ax;
            let aby = by - ay;
            let ab_len_sq = abx * abx + aby * aby;
            
            if ab_len_sq < 1.0 {
                continue; // Edge too short
            }
            
            let apx = node_cx - ax;
            let apy = node_cy - ay;
            
            let t = ((apx * abx + apy * aby) / ab_len_sq).clamp(0.0, 1.0);
            
            let closest_x = ax + t * abx;
            let closest_y = ay + t * aby;
            
            let dx = node_cx - closest_x;
            let dy = node_cy - closest_y;
            let dist = (dx * dx + dy * dy).sqrt();
            
            // If node is too close to edge, push it away
            if dist < node_radius * 2.0 && dist > 0.1 {
                let push = (node_radius * 2.0 - dist) * 0.3;
                total_dx += (dx / dist) * push;
                total_dy += (dy / dist) * push;
                push_count += 1;
            }
        }
        
        if push_count > 0 {
            movements.insert(nid, (total_dx / push_count as f64, total_dy / push_count as f64));
        }
    }
    
    // Apply movements (sorted for determinism)
    let mut sorted_movements: Vec<(NodeId, (f64, f64))> = movements.into_iter().collect();
    sorted_movements.sort_by_key(|(nid, _)| nid.0);
    
    for (nid, (dx, dy)) in sorted_movements {
        if let Some(pos) = positions.get_mut(&nid) {
            pos.x += dx as i32;
            pos.y += dy as i32;
        }
    }
}

/// Apply anchoring force: gently pull nodes back toward initial positions.
/// This reduces the "butterfly effect" where small changes cause large layout shifts.
fn apply_anchoring(
    positions: &mut HashMap<NodeId, PointI>,
    initial_positions: &HashMap<NodeId, PointI>,
    fixed_positions: &HashMap<NodeId, PointI>,
    strength: f64,
) {
    let fixed_set: HashSet<NodeId> = fixed_positions.keys().copied().collect();
    
    for (&nid, initial_pos) in initial_positions {
        if fixed_set.contains(&nid) {
            continue;
        }
        
        if let Some(pos) = positions.get_mut(&nid) {
            let dx = (initial_pos.x - pos.x) as f64;
            let dy = (initial_pos.y - pos.y) as f64;
            
            pos.x += (dx * strength) as i32;
            pos.y += (dy * strength) as i32;
        }
    }
}

/// Apply attraction force: pull connected nodes toward each other.
/// Edge weight affects attraction strength (inheritance > composition > association).
fn apply_attraction(
    positions: &mut HashMap<NodeId, PointI>,
    node_sizes: &HashMap<NodeId, SizeI>,
    diagram: &Diagram,
    adjacency: &Adjacency,
    fixed_positions: &HashMap<NodeId, PointI>,
    gap: i32,
) {
    let fixed_set: HashSet<NodeId> = fixed_positions.keys().copied().collect();
    
    // Build edge weight map: (node_a, node_b) -> max weight between them
    let mut edge_weights: HashMap<(NodeId, NodeId), i32> = HashMap::new();
    for edge in &diagram.edges {
        let weight = get_edge_weight(&edge.arrow);
        let key1 = (edge.from, edge.to);
        let key2 = (edge.to, edge.from);
        edge_weights.entry(key1).and_modify(|w| *w = (*w).max(weight)).or_insert(weight);
        edge_weights.entry(key2).and_modify(|w| *w = (*w).max(weight)).or_insert(weight);
    }
    
    // Sort for determinism
    let mut node_list: Vec<NodeId> = positions.keys().copied().collect();
    node_list.sort_by_key(|nid| nid.0);
    
    // Collect movements first to avoid mutation during iteration
    let mut movements: HashMap<NodeId, (f64, f64)> = HashMap::new();
    
    for &nid in &node_list {
        if fixed_set.contains(&nid) {
            continue;
        }
        
        let pos = match positions.get(&nid) {
            Some(p) => *p,
            None => continue,
        };
        let size = node_sizes.get(&nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
        let my_cx = (pos.x + size.w / 2) as f64;
        let my_cy = (pos.y + size.h / 2) as f64;
        
        let mut total_dx = 0.0f64;
        let mut total_dy = 0.0f64;
        let mut total_weight = 0.0f64;
        
        // Pull toward connected neighbors (sorted for determinism)
        let mut neighbors: Vec<(NodeId, usize)> = adjacency.get_neighbors(nid).to_vec();
        neighbors.sort_by_key(|(nid, _)| nid.0);
        
        for (neighbor_nid, edge_count) in neighbors {
            let neighbor_pos = match positions.get(&neighbor_nid) {
                Some(p) => *p,
                None => continue,
            };
            let neighbor_size = node_sizes.get(&neighbor_nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
            let neighbor_cx = (neighbor_pos.x + neighbor_size.w / 2) as f64;
            let neighbor_cy = (neighbor_pos.y + neighbor_size.h / 2) as f64;
            
            let dx = neighbor_cx - my_cx;
            let dy = neighbor_cy - my_cy;
            let dist = (dx * dx + dy * dy).sqrt();
            
            // Ideal distance: just enough to not overlap + gap
            let ideal_dist = ((size.w + neighbor_size.w) / 2 + gap) as f64;
            
            // Stronger pull if connected to fixed node
            let is_fixed_neighbor = fixed_set.contains(&neighbor_nid);
            
            // Use edge weight for attraction strength (inheritance = 100, line = 20)
            let arrow_weight = *edge_weights.get(&(nid, neighbor_nid)).unwrap_or(&20) as f64;
            let normalized_weight = arrow_weight / 20.0; // 1.0 for lines, 5.0 for inheritance
            
            let base_strength = if is_fixed_neighbor { 0.8 } else { 0.4 };
            let weight = edge_count as f64 * normalized_weight * if is_fixed_neighbor { 3.0 } else { 1.0 };
            
            if dist > ideal_dist * 0.5 {
                // Pull toward neighbor (proportional to excess distance)
                let pull = (dist - ideal_dist).max(0.0) * base_strength + 
                           dist * 0.05; // Always some attraction
                total_dx += (dx / dist.max(1.0)) * pull * weight;
                total_dy += (dy / dist.max(1.0)) * pull * weight;
                total_weight += weight;
            }
        }
        
        if total_weight > 0.0 {
            movements.insert(nid, (total_dx / total_weight, total_dy / total_weight));
        }
    }
    
    // Apply movements (sorted for determinism)
    let mut sorted_movements: Vec<(NodeId, (f64, f64))> = movements.into_iter().collect();
    sorted_movements.sort_by_key(|(nid, _)| nid.0);
    
    for (nid, (dx, dy)) in sorted_movements {
        if let Some(pos) = positions.get_mut(&nid) {
            pos.x += dx as i32;
            pos.y += dy as i32;
        }
    }
}

/// Resolve overlapping nodes by pushing them apart radially.
fn resolve_radial_overlaps(
    positions: &mut HashMap<NodeId, PointI>,
    node_sizes: &HashMap<NodeId, SizeI>,
    gap: i32,
    fixed_positions: &HashMap<NodeId, PointI>,
) {
    use super::RectI;
    
    let fixed_set: HashSet<NodeId> = fixed_positions.keys().copied().collect();
    
    // IMPORTANT: Sort for determinism!
    let mut node_list: Vec<NodeId> = positions.keys().copied().collect();
    node_list.sort_by_key(|nid| nid.0);
    
    // Multiple passes with damping to prevent explosion
    for pass in 0..20 {
        let mut any_moved = false;
        let damping = 0.5 / (1.0 + pass as f64 * 0.1); // Decreasing force each pass
        
        for &nid in &node_list {
            // Don't move fixed nodes
            if fixed_set.contains(&nid) {
                continue;
            }
            
            let pos = match positions.get(&nid) {
                Some(p) => *p,
                None => continue,
            };
            let size = node_sizes.get(&nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
            let rect = RectI { x: pos.x, y: pos.y, w: size.w, h: size.h };
            
            let mut total_dx = 0.0f64;
            let mut total_dy = 0.0f64;
            let mut overlap_count = 0;
            
            // Check against all other nodes
            for &other_nid in &node_list {
                if nid == other_nid {
                    continue;
                }
                
                let other_pos = match positions.get(&other_nid) {
                    Some(p) => *p,
                    None => continue,
                };
                let other_size = node_sizes.get(&other_nid).copied().unwrap_or(SizeI { w: 100, h: 80 });
                let other_rect = RectI { 
                    x: other_pos.x - gap, 
                    y: other_pos.y - gap, 
                    w: other_size.w + gap * 2, 
                    h: other_size.h + gap * 2 
                };
                
                if rect.overlaps(&other_rect) {
                    // Push away from the other node
                    let my_cx = pos.x + size.w / 2;
                    let my_cy = pos.y + size.h / 2;
                    let other_cx = other_pos.x + other_size.w / 2;
                    let other_cy = other_pos.y + other_size.h / 2;
                    
                    let dx = (my_cx - other_cx) as f64;
                    let dy = (my_cy - other_cy) as f64;
                    
                    // Normalize and apply gentle push
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                    let min_dist = ((size.w + other_size.w) / 2 + gap) as f64;
                    let push = (min_dist - dist).max(0.0) * damping;
                    
                    if push > 0.0 {
                        total_dx += (dx / dist) * push;
                        total_dy += (dy / dist) * push;
                        overlap_count += 1;
                    }
                }
            }
            
            if overlap_count > 0 && (total_dx.abs() > 1.0 || total_dy.abs() > 1.0) {
                let new_pos = PointI {
                    x: pos.x + total_dx as i32,
                    y: pos.y + total_dy as i32,
                };
                positions.insert(nid, new_pos);
                any_moved = true;
            }
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

    fn make_star_diagram() -> Diagram {
        // A is center, B, C, D are leaves
        //     B
        //     |
        // C - A - D
        Diagram {
            root: GroupId(0),
            groups: vec![Group {
                gid: GroupId(0),
                id: None,
                parent: None,
                pos: None,
                children_groups: vec![],
                children_nodes: vec![NodeId(0), NodeId(1), NodeId(2), NodeId(3)],
                order: 0,
            }],
            nodes: vec![
                Node { nid: NodeId(0), kind: "class".to_string(), modifiers: vec![], id: Ident("A".to_string()), label: None, group: GroupId(0), pos: None, width: None, height: None, body_lines: vec![], explicit: true, order: 0 },
                Node { nid: NodeId(1), kind: "class".to_string(), modifiers: vec![], id: Ident("B".to_string()), label: None, group: GroupId(0), pos: None, width: None, height: None, body_lines: vec![], explicit: true, order: 1 },
                Node { nid: NodeId(2), kind: "class".to_string(), modifiers: vec![], id: Ident("C".to_string()), label: None, group: GroupId(0), pos: None, width: None, height: None, body_lines: vec![], explicit: true, order: 2 },
                Node { nid: NodeId(3), kind: "class".to_string(), modifiers: vec![], id: Ident("D".to_string()), label: None, group: GroupId(0), pos: None, width: None, height: None, body_lines: vec![], explicit: true, order: 3 },
            ],
            edges: vec![
                Edge { from: NodeId(0), to: NodeId(1), arrow: "line".to_string(), label: None, order: 4 },
                Edge { from: NodeId(0), to: NodeId(2), arrow: "line".to_string(), label: None, order: 5 },
                Edge { from: NodeId(0), to: NodeId(3), arrow: "line".to_string(), label: None, order: 6 },
            ],
        }
    }

    #[test]
    fn test_find_root_highest_degree() {
        let diagram = make_star_diagram();
        let adjacency = Adjacency::from_diagram(&diagram);
        let nodes = vec![NodeId(0), NodeId(1), NodeId(2), NodeId(3)];
        
        let root = find_root(&diagram, &nodes, &adjacency, &HashMap::new());
        
        // A (NodeId(0)) has 3 connections, should be root
        assert_eq!(root, NodeId(0));
    }

    #[test]
    fn test_build_spanning_tree() {
        let diagram = make_star_diagram();
        let adjacency = Adjacency::from_diagram(&diagram);
        let nodes = vec![NodeId(0), NodeId(1), NodeId(2), NodeId(3)];
        
        let tree = build_spanning_tree(&diagram, &nodes, &adjacency, &HashMap::new());
        
        // Root should be A
        assert_eq!(tree.root, NodeId(0));
        
        // A should have 3 children
        let root_node = tree.nodes.get(&NodeId(0)).unwrap();
        assert_eq!(root_node.children.len(), 3);
        
        // B, C, D should have no children
        for nid in [NodeId(1), NodeId(2), NodeId(3)] {
            let node = tree.nodes.get(&nid).unwrap();
            assert!(node.children.is_empty());
        }
    }

    #[test]
    fn test_radial_positions_spread_out() {
        let diagram = make_star_diagram();
        let adjacency = Adjacency::from_diagram(&diagram);
        let nodes = vec![NodeId(0), NodeId(1), NodeId(2), NodeId(3)];
        let cfg = LayoutConfig::default();
        
        let node_sizes: HashMap<NodeId, SizeI> = nodes.iter()
            .map(|&nid| (nid, SizeI { w: 100, h: 80 }))
            .collect();
        
        let mut tree = build_spanning_tree(&diagram, &nodes, &adjacency, &HashMap::new());
        let root_nid = tree.root;
        assign_angles(&mut tree, root_nid, 0.0, 2.0 * std::f64::consts::PI);
        
        let positions = compute_radial_positions(&tree, &node_sizes, &cfg, &HashMap::new());
        
        // All nodes should have positions
        assert_eq!(positions.len(), 4);
        
        // Children should be spread out (different positions)
        let pos_b = positions.get(&NodeId(1)).unwrap();
        let pos_c = positions.get(&NodeId(2)).unwrap();
        let pos_d = positions.get(&NodeId(3)).unwrap();
        
        // They shouldn't all be at the same position
        assert!(pos_b != pos_c || pos_c != pos_d);
    }
}
