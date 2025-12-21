// Hierarchical (Layered) Layout Algorithm
//
// Replaces the previous greedy placement with a structured Layered Graph approach (Sugiyama-style).
// Goals:
// - Structure: Nodes align in clearer rows/columns (ranks).
// - Flow: Dependency direction (Inheritance, Compositions) flows Top-Down (or Left-Right).
// - Simplicity: Strict grid placement, no complex 2D searching.

use std::collections::{HashMap, HashSet, VecDeque};
use crate::parser::{PointI, Diagram, GroupId, NodeId};
use super::{RectI, LayoutConfig};
use super::spatial_grid::SpatialGrid;
use super::adjacency::Adjacency;

/// Layout children of a group using Hierarchical Placement.
pub fn layout_group_children_graph_driven(
    diagram: &Diagram,
    gid: GroupId,
    cfg: &LayoutConfig,
    _adjacency: &Adjacency,
    group_local_pos: &mut HashMap<GroupId, PointI>,
    node_local_pos: &mut HashMap<NodeId, PointI>,
    group_local_bounds: &HashMap<GroupId, RectI>,
) {
    let g = &diagram.groups[gid.0];

    // 1. Separate fixed and free items
    let mut fixed_groups: Vec<GroupId> = Vec::new();
    let mut free_groups: Vec<GroupId> = Vec::new();
    let mut fixed_nodes: Vec<NodeId> = Vec::new();
    let mut free_nodes: Vec<NodeId> = Vec::new();

    for &cgid in &g.children_groups {
        if diagram.groups[cgid.0].pos.is_some() {
            fixed_groups.push(cgid);
        } else {
            free_groups.push(cgid);
        }
    }

    for &nid in &g.children_nodes {
        if diagram.nodes[nid.0].pos.is_some() {
            fixed_nodes.push(nid);
        } else {
            free_nodes.push(nid);
        }
    }

    // 2. Assign fixed positions
    for cgid in &fixed_groups {
        let p = diagram.groups[cgid.0].pos.unwrap();
        group_local_pos.insert(*cgid, p);
    }
    for nid in &fixed_nodes {
        let p = diagram.nodes[nid.0].pos.unwrap();
        node_local_pos.insert(*nid, p);
    }

    // 3. Initialize spatial grid with fixed items
    let cell_size = cfg.node_size.w.max(cfg.node_size.h);
    let mut spatial = SpatialGrid::new(cell_size);

    for cgid in &fixed_groups {
        let p = *group_local_pos.get(cgid).unwrap();
        let lb = group_local_bounds.get(cgid).copied().unwrap_or(RectI {
            x: 0, y: 0, w: cfg.min_group_size.w, h: cfg.min_group_size.h,
        });
        spatial.insert(RectI { x: p.x, y: p.y, w: lb.w, h: lb.h });
    }
    for nid in &fixed_nodes {
        let p = *node_local_pos.get(nid).unwrap();
        spatial.insert(RectI { x: p.x, y: p.y, w: cfg.node_size.w, h: cfg.node_size.h });
    }

    // 4. Place free groups FIRST (Hierarchical)
    // We treat groups as nodes in the hierarchy too.
    if !free_groups.is_empty() {
         layout_groups_hierarchical(
            diagram,
            &free_groups,
            cfg,
            &mut spatial,
            group_local_pos,
            group_local_bounds,
        );
    }

    // 5. Place free nodes (Hierarchical)
    if !free_nodes.is_empty() {
        layout_nodes_hierarchical(
             diagram,
             &free_nodes,
             &fixed_nodes,
             cfg,
             &mut spatial,
             node_local_pos,
             group_local_pos,
        );
    }
}

/// Helper function to get arrow direction for ranking.
/// Returns (parent_id, child_id) where parent should be above child.
fn get_edge_direction(arrow: &str, from: NodeId, to: NodeId) -> (NodeId, NodeId) {
    match arrow {
        "extends_right" => (to, from),     // B is parent of A
        "extends_left" => (from, to),      // A is parent of B
        "compose" | "aggregate" | "assoc_right" | "dep_right" => (from, to), // A owns/calls B
        "assoc_left" | "dep_left" => (to, from), // B owns/calls A
        _ => (from, to), // Default
    }
}

/// Layout groups using a hierarchical approach.
fn layout_groups_hierarchical(
    diagram: &Diagram,
    groups: &[GroupId],
    cfg: &LayoutConfig,
    spatial: &mut SpatialGrid,
    group_local_pos: &mut HashMap<GroupId, PointI>,
    group_local_bounds: &HashMap<GroupId, RectI>,
) {
    // Build Group Dependency Graph
    // Flatten edges to see connectivity between groups
    let mut adj: HashMap<GroupId, Vec<GroupId>> = HashMap::new();
    let mut rev_adj: HashMap<GroupId, Vec<GroupId>> = HashMap::new();
    let group_set: HashSet<GroupId> = groups.iter().copied().collect();

    // Map Node -> Group
    let mut node_to_group: HashMap<NodeId, GroupId> = HashMap::new();
    for &gid in groups {
        for &nid in &diagram.groups[gid.0].children_nodes {
            node_to_group.insert(nid, gid);
        }
    }

    for edge in &diagram.edges {
        let (parent_nid, child_nid) = get_edge_direction(&edge.arrow, edge.from, edge.to);

        if let (Some(&pgid), Some(&cgid)) = (node_to_group.get(&parent_nid), node_to_group.get(&child_nid)) {
            if pgid != cgid && group_set.contains(&pgid) && group_set.contains(&cgid) {
                // Group Parent -> Group Child
                adj.entry(pgid).or_default().push(cgid);
                rev_adj.entry(cgid).or_default().push(pgid);
            }
        }
    }

    // Assign Ranks (BFS)
    let mut ranks: HashMap<GroupId, i32> = HashMap::new();
    let mut visited: HashSet<GroupId> = HashSet::new();
    let mut queue: VecDeque<(GroupId, i32)> = VecDeque::new();

    // Roots
    for &gid in groups {
        if !rev_adj.contains_key(&gid) {
            queue.push_back((gid, 0));
            visited.insert(gid);
        }
    }
    if queue.is_empty() && !groups.is_empty() {
        let first = groups.iter().min_by_key(|g| diagram.groups[g.0].order).unwrap();
        queue.push_back((*first, 0));
        visited.insert(*first);
    }
    
    while let Some((gid, rank)) = queue.pop_front() {
        ranks.insert(gid, rank);
         if let Some(neighbors) = adj.get(&gid) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) && groups.contains(&neighbor) {
                    visited.insert(neighbor);
                    queue.push_back((neighbor, rank + 1));
                }
            }
        }
    }
    
    // Handle unvisited
    for &gid in groups {
         if !visited.contains(&gid) {
            ranks.insert(gid, 0);
         }
    }

    // Group by rank
    let mut rank_map: HashMap<i32, Vec<GroupId>> = HashMap::new();
    for &gid in groups {
        if let Some(&r) = ranks.get(&gid) {
            rank_map.entry(r).or_default().push(gid);
        }
    }

    let start_x = cfg.group_padding;
    let start_y = cfg.group_padding;
    let mut next_y = start_y;

    let max_rank = ranks.values().max().copied().unwrap_or(0);

    // Minimize Edge Crossings (Barycenter) for Groups
    let iterations = 10;
    for _ in 0..iterations {
        // Down-sweep
        for r in 1..=max_rank {
            if let Some(prev_row_groups) = rank_map.get(&(r - 1)).cloned() {
                let prev_pos: HashMap<GroupId, usize> = prev_row_groups.iter().enumerate().map(|(i, &g)| (g, i)).collect();
                if let Some(row_groups) = rank_map.get_mut(&r) {
                    row_groups.sort_by(|&a, &b| {
                        let avg_a = get_group_barycenter(a, &rev_adj, &prev_pos);
                        let avg_b = get_group_barycenter(b, &rev_adj, &prev_pos);
                        avg_a.partial_cmp(&avg_b).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }
        // Up-sweep
        for r in (0..max_rank).rev() {
            if let Some(next_row_groups) = rank_map.get(&(r + 1)).cloned() {
                let next_pos: HashMap<GroupId, usize> = next_row_groups.iter().enumerate().map(|(i, &g)| (g, i)).collect();
                if let Some(row_groups) = rank_map.get_mut(&r) {
                    row_groups.sort_by(|&a, &b| {
                        let avg_a = get_group_barycenter(a, &adj, &next_pos);
                        let avg_b = get_group_barycenter(b, &adj, &next_pos);
                        avg_a.partial_cmp(&avg_b).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }
    }

    for r in 0..=max_rank {
        if let Some(row_groups) = rank_map.get_mut(&r) {
            let mut current_x = start_x;
            let _row_start_y = next_y;
            let mut current_y = next_y;
            let limit_w = if cfg.max_row_w > 0 { cfg.max_row_w } else { 1200 };

            for &gid in row_groups.iter() {
                 let bounds = group_local_bounds.get(&gid).copied().unwrap_or(RectI { 
                     x: 0, y: 0, w: cfg.min_group_size.w, h: cfg.min_group_size.h 
                });

                 // Check wrap
                 if current_x + bounds.w > limit_w && current_x > start_x {
                     current_x = start_x;
                     current_y += 300;
                 }

                 let mut pos = PointI { x: current_x, y: current_y };
                 
                 // Overlap check and shift
                 let mut rect = RectI { x: pos.x, y: pos.y, w: bounds.w, h: bounds.h };
                 while spatial.overlaps_any(&rect) {
                     pos.x += 50;
                     if pos.x + bounds.w > limit_w {
                         pos.x = start_x;
                         pos.y += 100;
                     }
                     rect = RectI { x: pos.x, y: pos.y, w: bounds.w, h: bounds.h };
                 }
                 
                 // Update max Y for next rank
                 if pos.y + bounds.h + cfg.gap * 2 > next_y {
                     next_y = pos.y + bounds.h + cfg.gap * 2;
                 }
                 
                 group_local_pos.insert(gid, pos);
                 spatial.insert(rect);
                 
                 current_x = pos.x + bounds.w + cfg.gap;
                 if pos.y > current_y { current_y = pos.y; }
            }
        }
    }
}

/// Layout nodes using strict Hierarchical (Ranked) approach.
fn layout_nodes_hierarchical(
    diagram: &Diagram,
    nodes: &[NodeId],
    fixed_nodes: &[NodeId],
    cfg: &LayoutConfig,
    spatial: &mut SpatialGrid,
    node_local_pos: &mut HashMap<NodeId, PointI>,
    _group_local_pos: &HashMap<GroupId, PointI>,
) {
    // 1. Build Dependency Graph
    let mut all_nodes = Vec::with_capacity(nodes.len() + fixed_nodes.len());
    all_nodes.extend_from_slice(nodes);
    all_nodes.extend_from_slice(fixed_nodes);

    let (adj, rev_adj) = build_dependency_graph(diagram, &all_nodes);

    // 2. Assign Ranks
    let mut ranks: HashMap<NodeId, i32> = HashMap::new();
    let mut visited: HashSet<NodeId> = HashSet::new();
    let mut queue: VecDeque<(NodeId, i32)> = VecDeque::new();

    let layer_height = cfg.node_size.h + cfg.gap * 2;

    // Seed Fixed Nodes as Visited/Ranked based on Y position
    for &fnid in fixed_nodes {
        if let Some(pos) = node_local_pos.get(&fnid) {
            let r = (pos.y as f32 / layer_height as f32).max(0.0) as i32;
            ranks.insert(fnid, r);
            visited.insert(fnid);
            queue.push_back((fnid, r));
        }
    }

    // Find roots among FREE nodes
    for &nid in nodes {
        if !visited.contains(&nid) {
             if !rev_adj.contains_key(&nid) {
                 queue.push_back((nid, 0));
                 visited.insert(nid);
             }
        }
    }
    
    if queue.is_empty() {
         for &nid in nodes {
             if !visited.contains(&nid) {
                 queue.push_back((nid, 0));
                 visited.insert(nid);
                 break;
             }
         }
    }

    while let Some((nid, rank)) = queue.pop_front() {
        if !fixed_nodes.contains(&nid) {
             ranks.insert(nid, rank);
        }
        
        if let Some(neighbors) = adj.get(&nid) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) && all_nodes.contains(&neighbor) {
                    visited.insert(neighbor);
                    queue.push_back((neighbor, rank + 1));
                }
            }
        }
    }

    // Handle remaining unvisited (cycles or disconnected)
    for &nid in nodes {
        if !visited.contains(&nid) {
             let mut sub_queue = VecDeque::new();
             sub_queue.push_back((nid, 0));
             visited.insert(nid);
             while let Some((snid, srank)) = sub_queue.pop_front() {
                 if !fixed_nodes.contains(&snid) {
                    ranks.insert(snid, srank);
                 }
                if let Some(neighbors) = adj.get(&snid) {
                    for &neighbor in neighbors {
                        if !visited.contains(&neighbor) && all_nodes.contains(&neighbor) {
                            visited.insert(neighbor);
                            sub_queue.push_back((neighbor, srank + 1));
                        }
                    }
                }
             }
        }
    }

    let max_rank = ranks.values().max().copied().unwrap_or(0);

    // 3. Group by Rank (only FREE nodes)
    let mut rank_map: HashMap<i32, Vec<NodeId>> = HashMap::new();
    for &nid in nodes {
        if let Some(&r) = ranks.get(&nid) {
            rank_map.entry(r).or_default().push(nid);
        }
    }
    
    // 3.5. Minimize Edge Crossings (Barycenter Heuristic)
    let iterations = 12;
    for _ in 0..iterations {
        // Down-sweep
        for r in 1..=max_rank {
            if let Some(prev_row_nodes) = rank_map.get(&(r - 1)).cloned() { 
                let prev_pos: HashMap<NodeId, usize> = prev_row_nodes.iter().enumerate().map(|(i, &n)| (n, i)).collect();
                
                if let Some(row_nodes) = rank_map.get_mut(&r) {
                    row_nodes.sort_by(|&a, &b| {
                        let avg_a = get_barycenter(a, &rev_adj, &prev_pos);
                        let avg_b = get_barycenter(b, &rev_adj, &prev_pos);
                        avg_a.partial_cmp(&avg_b).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }

        // Up-sweep
        for r in (0..max_rank).rev() {
            if let Some(next_row_nodes) = rank_map.get(&(r + 1)).cloned() {
                let next_pos: HashMap<NodeId, usize> = next_row_nodes.iter().enumerate().map(|(i, &n)| (n, i)).collect();

                if let Some(row_nodes) = rank_map.get_mut(&r) {
                    row_nodes.sort_by(|&a, &b| {
                        let avg_a = get_barycenter(a, &adj, &next_pos);
                        let avg_b = get_barycenter(b, &adj, &next_pos);
                        avg_a.partial_cmp(&avg_b).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }
    }

    // 4. Assign Grid Positions
    let start_x = cfg.group_padding;
    let start_y = cfg.group_padding;
    let col_width = cfg.node_size.w + cfg.gap;
    let row_height = cfg.node_size.h + cfg.gap * 2;

    let mut next_y = start_y;

    for r in 0..=max_rank {
        if let Some(row_nodes) = rank_map.get_mut(&r) {
            let rank_start_y = next_y;
            let mut current_y = rank_start_y;
            let mut current_x = start_x;
            
            let limit_w = if cfg.max_row_w > 0 { cfg.max_row_w } else { 1200 };

            for &nid in row_nodes.iter() {
                 let mut pos = PointI { x: current_x, y: current_y };
                 
                 // Check wrapping
                 if current_x + col_width > limit_w && current_x > start_x {
                     current_x = start_x;
                     current_y += row_height;
                     pos.x = current_x;
                     pos.y = current_y;
                 }
                 
                 // Check overlap (backup)
                 let mut safe_pos = pos;
                 let mut rect = RectI { x: safe_pos.x, y: safe_pos.y, w: cfg.node_size.w, h: cfg.node_size.h };
                 while spatial.overlaps_any(&rect) {
                     safe_pos.x += col_width;
                      if safe_pos.x + col_width > limit_w {
                         safe_pos.x = start_x;
                         safe_pos.y += row_height;
                      }
                      rect = RectI { x: safe_pos.x, y: safe_pos.y, w: cfg.node_size.w, h: cfg.node_size.h };
                 }

                 current_x = safe_pos.x + col_width;
                 current_y = safe_pos.y;
                 
                 node_local_pos.insert(nid, safe_pos);
                 spatial.insert(RectI { x: safe_pos.x, y: safe_pos.y, w: cfg.node_size.w, h: cfg.node_size.h });
            }
            next_y = current_y + row_height;
        }
    }
}

/// Build directed adjacency for hierarchy.
fn build_dependency_graph(diagram: &Diagram, scope: &[NodeId]) -> (HashMap<NodeId, Vec<NodeId>>, HashMap<NodeId, Vec<NodeId>>) {
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

/// Calculate average position (barycenter) of connected nodes in the adjacent rank.
fn get_barycenter(
    nid: NodeId,
    adj: &HashMap<NodeId, Vec<NodeId>>,
    neighbor_pos: &HashMap<NodeId, usize>,
) -> f32 {
    if let Some(neighbors) = adj.get(&nid) {
        let mut sum = 0.0;
        let mut count = 0.0;
        for neighbor in neighbors {
            if let Some(&pos) = neighbor_pos.get(neighbor) {
                sum += pos as f32;
                count += 1.0;
            }
        }
        if count > 0.0 {
            return sum / count;
        }
    }
    0.0
}

fn get_group_barycenter(
    gid: GroupId,
    adj: &HashMap<GroupId, Vec<GroupId>>,
    neighbor_pos: &HashMap<GroupId, usize>,
) -> f32 {
    if let Some(neighbors) = adj.get(&gid) {
        let mut sum = 0.0;
        let mut count = 0.0;
        for neighbor in neighbors {
            if let Some(&pos) = neighbor_pos.get(neighbor) {
                sum += pos as f32;
                count += 1.0;
            }
        }
        if count > 0.0 {
            return sum / count;
        }
    }
    0.0
}
