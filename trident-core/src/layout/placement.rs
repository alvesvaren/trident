// Graph-driven placement algorithm.
//
// Places nodes based on connectivity rather than simple row-packing.
// Connected nodes are placed closer together, reducing edge lengths.

use std::collections::{HashMap, HashSet};
use crate::parser::{PointI, Diagram, GroupId, ClassId};
use super::{SizeI, RectI, LayoutConfig};
use super::spatial_grid::SpatialGrid;
use super::adjacency::{Adjacency, NodeWeights, compute_node_weights};

/// Number of candidate positions to try per node.
const NUM_CANDIDATES: usize = 32;

/// Layout children of a group using graph-driven placement.
pub fn layout_group_children_graph_driven(
    diagram: &Diagram,
    gid: GroupId,
    cfg: &LayoutConfig,
    adjacency: &Adjacency,
    group_local_pos: &mut HashMap<GroupId, PointI>,
    class_local_pos: &mut HashMap<ClassId, PointI>,
    group_local_bounds: &HashMap<GroupId, RectI>,
) {
    let g = &diagram.groups[gid.0];

    // Separate fixed and free items
    let mut fixed_groups: Vec<GroupId> = Vec::new();
    let mut free_groups: Vec<GroupId> = Vec::new();
    let mut fixed_classes: Vec<ClassId> = Vec::new();
    let mut free_classes: Vec<ClassId> = Vec::new();

    for &cgid in &g.children_groups {
        if diagram.groups[cgid.0].pos.is_some() {
            fixed_groups.push(cgid);
        } else {
            free_groups.push(cgid);
        }
    }

    for &cid in &g.children_classes {
        if diagram.classes[cid.0].pos.is_some() {
            fixed_classes.push(cid);
        } else {
            free_classes.push(cid);
        }
    }

    // Assign fixed positions
    for cgid in &fixed_groups {
        let p = diagram.groups[cgid.0].pos.unwrap();
        group_local_pos.insert(*cgid, p);
    }
    for cid in &fixed_classes {
        let p = diagram.classes[cid.0].pos.unwrap();
        class_local_pos.insert(*cid, p);
    }

    // Initialize spatial grid with fixed items
    let cell_size = cfg.class_size.w.max(cfg.class_size.h);
    let mut spatial = SpatialGrid::new(cell_size);

    for cgid in &fixed_groups {
        let p = *group_local_pos.get(cgid).unwrap();
        let lb = group_local_bounds.get(cgid).copied().unwrap_or(RectI {
            x: 0, y: 0, w: cfg.min_group_size.w, h: cfg.min_group_size.h,
        });
        spatial.insert(RectI { x: p.x, y: p.y, w: lb.w, h: lb.h });
    }
    for cid in &fixed_classes {
        let p = *class_local_pos.get(cid).unwrap();
        spatial.insert(RectI { x: p.x, y: p.y, w: cfg.class_size.w, h: cfg.class_size.h });
    }

    // Compute node weights for boundary bias
    let node_weights = compute_node_weights(diagram, gid, adjacency);

    // Track placed items and their positions
    let mut placed_classes: HashSet<ClassId> = fixed_classes.iter().copied().collect();
    let mut placed_groups: HashSet<GroupId> = fixed_groups.iter().copied().collect();
    let mut class_positions: HashMap<ClassId, PointI> = class_local_pos
        .iter()
        .filter(|(cid, _)| placed_classes.contains(cid))
        .map(|(k, v)| (*k, *v))
        .collect();

    // Sort free items by declaration order for determinism
    free_classes.sort_by_key(|cid| diagram.classes[cid.0].order);
    free_groups.sort_by_key(|gid| diagram.groups[gid.0].order);

    // 1. Place free groups FIRST so their positions are known
    place_free_groups(
        diagram,
        &free_groups,
        cfg,
        &mut spatial,
        group_local_pos,
        &mut placed_groups,
        group_local_bounds,
    );

    // 2. Build context for class placement: include nodes inside placed groups (projected to current coords)
    // This allows root classes to place themselves near the groups they connect to.
    let mut context_placed_classes = placed_classes.clone();
    let mut context_class_positions = class_positions.clone();

    // Add all nodes from all placed child groups to the context
    for &child_gid in &placed_groups {
        if let Some(group_pos) = group_local_pos.get(&child_gid) {
             // For each class in this child group
             for &child_cid in &diagram.groups[child_gid.0].children_classes {
                 if let Some(child_local_pos) = class_local_pos.get(&child_cid) {
                     // Project position to current group coordinates
                     let projected_pos = PointI {
                         x: group_pos.x + child_local_pos.x,
                         y: group_pos.y + child_local_pos.y,
                     };
                     context_placed_classes.insert(child_cid);
                     context_class_positions.insert(child_cid, projected_pos);
                 }
             }
        }
    }

    // 3. Place free classes using graph-driven algorithm
    let mut pending_classes: Vec<ClassId> = free_classes.clone();

    while !pending_classes.is_empty() {
        // Pick next node based on context
        let next_idx = pick_next_class(
            &pending_classes,
            &context_placed_classes, // Use context with projected nodes
            adjacency,
            diagram,
        );
        let next_cid = pending_classes.remove(next_idx);

        let sz = cfg.class_size;
        let weights = node_weights.get(&next_cid);

        // Generate candidate positions
        let candidates = generate_candidates(
            next_cid,
            &context_class_positions, // Use context positions
            adjacency,
            cfg,
            &context_placed_classes,
        );

        // Find best non-overlapping candidate
        let pos = find_best_position(
            &candidates,
            sz,
            &spatial,
            next_cid,
            &context_class_positions,
            adjacency,
            weights,
            cfg,
        );

        // Place the node
        class_local_pos.insert(next_cid, pos);
        class_positions.insert(next_cid, pos);
        placed_classes.insert(next_cid);
        spatial.insert(RectI { x: pos.x, y: pos.y, w: sz.w, h: sz.h });

        // Also update context for subsequent nodes
        context_placed_classes.insert(next_cid);
        context_class_positions.insert(next_cid, pos);
    }

    // Optimization pass: try to improve positions (compact and reduce edge lengths)
    optimize_node_positions(
        &free_classes,
        class_local_pos,
        cfg,
        adjacency,
        &fixed_classes,
        &fixed_groups,
        group_local_bounds,
    );

    optimize_group_positions(
        &free_groups,
        group_local_pos,
        cfg,
        diagram,
        group_local_bounds,
        &fixed_classes,
        class_local_pos,
    );
}

/// Pick the next class to place based on connectivity to already-placed nodes.
/// When no nodes are placed yet, picks the highest-degree node (hub) to start from.
fn pick_next_class(
    pending: &[ClassId],
    placed: &HashSet<ClassId>,
    adjacency: &Adjacency,
    diagram: &Diagram,
) -> usize {
    let mut best_idx = 0;
    let mut best_score = 0usize;
    let mut best_degree = 0usize;
    let mut best_order = usize::MAX;

    for (idx, &cid) in pending.iter().enumerate() {
        // Score = sum of edge weights to placed nodes
        let mut score = 0usize;
        for (neighbor, weight) in adjacency.get_neighbors(cid) {
            if placed.contains(neighbor) {
                score += weight;
            }
        }

        // Total degree used as tiebreaker (prefer hub nodes)
        let degree = adjacency.get_degree(cid);
        let order = diagram.classes[cid.0].order;

        // Priority: 1) most connections to placed, 2) highest total degree, 3) earlier order
        let is_better = score > best_score
            || (score == best_score && degree > best_degree)
            || (score == best_score && degree == best_degree && order < best_order);

        if is_better {
            best_idx = idx;
            best_score = score;
            best_degree = degree;
            best_order = order;
        }
    }

    best_idx
}

/// Generate candidate positions around connected placed nodes.
/// Generates positions adjacent to ALL connected placed nodes, not just the strongest.
fn generate_candidates(
    cid: ClassId,
    placed_positions: &HashMap<ClassId, PointI>,
    adjacency: &Adjacency,
    cfg: &LayoutConfig,
    placed: &HashSet<ClassId>,
) -> Vec<PointI> {
    let mut candidates = Vec::with_capacity(NUM_CANDIDATES * 2);
    let step_x = cfg.class_size.w + cfg.gap;
    let step_y = cfg.class_size.h + cfg.gap;

    // Collect all connected placed neighbors
    let mut connected_neighbors: Vec<(ClassId, usize, PointI)> = Vec::new();
    for (neighbor, weight) in adjacency.get_neighbors(cid) {
        if let Some(&pos) = placed_positions.get(neighbor) {
            if placed.contains(neighbor) {
                connected_neighbors.push((*neighbor, *weight, pos));
            }
        }
    }

    // Sort by weight descending so we prioritize positions near strongly connected nodes
    connected_neighbors.sort_by_key(|(_, w, _)| std::cmp::Reverse(*w));

    // For each connected neighbor, generate adjacent positions
    for (_, _weight, neighbor_pos) in &connected_neighbors {
        // Generate 4 cardinal positions directly adjacent
        let adjacent = [
            // Right
            PointI { x: neighbor_pos.x + step_x, y: neighbor_pos.y },
            // Left
            PointI { x: neighbor_pos.x - step_x, y: neighbor_pos.y },
            // Below
            PointI { x: neighbor_pos.x, y: neighbor_pos.y + step_y },
            // Above
            PointI { x: neighbor_pos.x, y: neighbor_pos.y - step_y },
            // Diagonals (less preferred but still close)
            PointI { x: neighbor_pos.x + step_x, y: neighbor_pos.y + step_y },
            PointI { x: neighbor_pos.x - step_x, y: neighbor_pos.y + step_y },
            PointI { x: neighbor_pos.x + step_x, y: neighbor_pos.y - step_y },
            PointI { x: neighbor_pos.x - step_x, y: neighbor_pos.y - step_y },
        ];

        for pos in adjacent {
            if pos.x >= cfg.group_padding && pos.y >= cfg.group_padding {
                // Avoid duplicates
                if !candidates.contains(&pos) {
                    candidates.push(pos);
                }
            }
        }

        if candidates.len() >= NUM_CANDIDATES {
            break;
        }
    }

    // If we have connected neighbors, also add positions in a wider ring
    if !connected_neighbors.is_empty() {
        let center = connected_neighbors[0].2; // Strongest neighbor
        for ring in 2..=3 {
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if dx == 0 && dy == 0 { continue; }
                    let pos = PointI {
                        x: center.x + dx * step_x * ring,
                        y: center.y + dy * step_y * ring,
                    };
                    if pos.x >= cfg.group_padding && pos.y >= cfg.group_padding {
                        if !candidates.contains(&pos) {
                            candidates.push(pos);
                        }
                    }
                }
            }
        }
    }

    // If no placed neighbors, use grid positions
    if candidates.is_empty() {
        generate_grid_candidates(&mut candidates, cfg, NUM_CANDIDATES);
    }

    candidates
}

/// Generate grid positions (fallback for nodes with no placed neighbors).
fn generate_grid_candidates(candidates: &mut Vec<PointI>, cfg: &LayoutConfig, count: usize) {
    let step_x = cfg.class_size.w + cfg.gap;
    let step_y = cfg.class_size.h + cfg.gap;
    let cols = (cfg.max_row_w / step_x).max(1);

    for i in 0..count {
        let col = i as i32 % cols;
        let row = i as i32 / cols;
        let pos = PointI {
            x: cfg.group_padding + col * step_x,
            y: cfg.group_padding + row * step_y,
        };
        if !candidates.contains(&pos) {
            candidates.push(pos);
        }
    }
}


/// Find the best position from candidates (lowest edge length score, non-overlapping).
fn find_best_position(
    candidates: &[PointI],
    size: SizeI,
    spatial: &SpatialGrid,
    cid: ClassId,
    placed_positions: &HashMap<ClassId, PointI>,
    adjacency: &Adjacency,
    weights: Option<&NodeWeights>,
    cfg: &LayoutConfig,
) -> PointI {
    let mut best_pos = PointI { x: cfg.group_padding, y: cfg.group_padding };
    let mut best_score = i64::MAX;

    for &pos in candidates {
        let rect = RectI { x: pos.x, y: pos.y, w: size.w, h: size.h };

        // Skip overlapping positions
        if spatial.overlaps_any(&rect) {
            continue;
        }

        // Score = sum of squared distances to connected placed nodes
        let mut score: i64 = 0;
        for (neighbor, weight) in adjacency.get_neighbors(cid) {
            if let Some(&neighbor_pos) = placed_positions.get(neighbor) {
                let dx = (pos.x - neighbor_pos.x) as i64;
                let dy = (pos.y - neighbor_pos.y) as i64;
                // Weighted squared distance (prefer closer = lower score)
                score += (dx * dx + dy * dy) * (*weight as i64);
            }
        }

        // Apply boundary bias for nodes with external edges
        if let Some(w) = weights {
            if w.w_out > 0 {
                // Nodes with external edges should be near boundary
                // Slightly reduce score for positions near edges
                // (For now, no group bounds available, skip this bias)
            }
        }

        if score < best_score {
            best_score = score;
            best_pos = pos;
        }
    }

    // If no valid candidate (all overlapping), fallback to row-packing style
    if best_score == i64::MAX {
        best_pos = find_first_free_position(cfg, spatial, size);
    }

    best_pos
}

/// Fallback: find first non-overlapping position via row scanning.
fn find_first_free_position(cfg: &LayoutConfig, spatial: &SpatialGrid, size: SizeI) -> PointI {
    let step = cfg.gap;
    let mut y = cfg.group_padding;

    loop {
        let mut x = cfg.group_padding;
        while x + size.w <= cfg.max_row_w {
            let rect = RectI { x, y, w: size.w, h: size.h };
            if !spatial.overlaps_any(&rect) {
                return PointI { x, y };
            }
            x += step;
        }
        y += step;

        // Safety valve to prevent infinite loop
        if y > 10000 {
            return PointI { x: cfg.group_padding, y: cfg.group_padding };
        }
    }
}

/// Place free groups using graph-driven placement based on inter-group edges.
/// Groups with many edges between them will be placed adjacent to each other.
fn place_free_groups(
    diagram: &Diagram,
    free_groups: &[GroupId],
    cfg: &LayoutConfig,
    spatial: &mut SpatialGrid,
    group_local_pos: &mut HashMap<GroupId, PointI>,
    placed_groups: &mut HashSet<GroupId>,
    group_local_bounds: &HashMap<GroupId, RectI>,
) {
    use super::adjacency::compute_group_adjacency;
    
    if free_groups.is_empty() {
        return;
    }

    // Build group-level adjacency: how many edges between each pair of groups
    let group_adj = compute_group_adjacency(diagram);

    // Build neighbor lists for groups
    let mut group_neighbors: HashMap<GroupId, Vec<(GroupId, usize)>> = HashMap::new();
    let mut group_degree: HashMap<GroupId, usize> = HashMap::new();

    for ((g1, g2), count) in &group_adj {
        group_neighbors.entry(*g1).or_default().push((*g2, *count));
        group_neighbors.entry(*g2).or_default().push((*g1, *count));
        *group_degree.entry(*g1).or_default() += count;
        *group_degree.entry(*g2).or_default() += count;
    }

    // Track placed group positions for candidate generation
    let mut group_positions: HashMap<GroupId, PointI> = group_local_pos
        .iter()
        .filter(|(gid, _)| placed_groups.contains(gid))
        .map(|(k, v)| (*k, *v))
        .collect();

    // Sort free groups by declaration order
    let mut pending: Vec<GroupId> = free_groups.to_vec();
    pending.sort_by_key(|gid| diagram.groups[gid.0].order);

    while !pending.is_empty() {
        // Pick next group: highest edge weight to placed groups, tie-break by order
        let next_idx = pick_next_group(&pending, placed_groups, &group_neighbors, diagram);
        let next_gid = pending.remove(next_idx);

        let lb = group_local_bounds.get(&next_gid).copied().unwrap_or(RectI {
            x: 0, y: 0, w: cfg.min_group_size.w, h: cfg.min_group_size.h,
        });
        let size = SizeI { w: lb.w, h: lb.h };

        // Generate candidate positions around connected placed groups
        let candidates = generate_group_candidates(
            next_gid,
            &group_positions,
            &group_neighbors,
            group_local_bounds,
            cfg,
            placed_groups,
        );

        // Find best non-overlapping candidate
        let pos = find_best_group_position(
            &candidates,
            size,
            spatial,
            next_gid,
            &group_positions,
            &group_neighbors,
            group_local_bounds,
            cfg,
        );

        // Place the group
        group_local_pos.insert(next_gid, pos);
        group_positions.insert(next_gid, pos);
        placed_groups.insert(next_gid);
        spatial.insert(RectI { x: pos.x, y: pos.y, w: size.w, h: size.h });
    }
}

/// Pick the next group to place based on connectivity to already-placed groups.
fn pick_next_group(
    pending: &[GroupId],
    placed: &HashSet<GroupId>,
    group_neighbors: &HashMap<GroupId, Vec<(GroupId, usize)>>,
    diagram: &Diagram,
) -> usize {
    let mut best_idx = 0;
    let mut best_score = 0usize;
    let mut best_order = usize::MAX;

    for (idx, &gid) in pending.iter().enumerate() {
        // Score = sum of edge weights to placed groups
        let mut score = 0usize;
        if let Some(neighbors) = group_neighbors.get(&gid) {
            for (neighbor, weight) in neighbors {
                if placed.contains(neighbor) {
                    score += weight;
                }
            }
        }

        // If no placed neighbors, use total degree
        if score == 0 && placed.is_empty() {
            score = group_neighbors.get(&gid)
                .map(|n| n.iter().map(|(_, w)| w).sum())
                .unwrap_or(0);
        }

        let order = diagram.groups[gid.0].order;

        // Higher score wins, tie-break by earlier order
        if score > best_score || (score == best_score && order < best_order) {
            best_idx = idx;
            best_score = score;
            best_order = order;
        }
    }

    best_idx
}

/// Generate candidate positions for a group around connected placed groups.
fn generate_group_candidates(
    gid: GroupId,
    placed_positions: &HashMap<GroupId, PointI>,
    group_neighbors: &HashMap<GroupId, Vec<(GroupId, usize)>>,
    group_local_bounds: &HashMap<GroupId, RectI>,
    cfg: &LayoutConfig,
    placed: &HashSet<GroupId>,
) -> Vec<PointI> {
    let mut candidates = Vec::with_capacity(NUM_CANDIDATES);

    // Find strongest connected placed neighbor
    let mut best_neighbor: Option<GroupId> = None;
    let mut best_weight = 0usize;

    if let Some(neighbors) = group_neighbors.get(&gid) {
        for (neighbor, weight) in neighbors {
            if placed.contains(neighbor) && *weight > best_weight {
                best_neighbor = Some(*neighbor);
                best_weight = *weight;
            }
        }
    }

    if let Some(neighbor) = best_neighbor {
        if let Some(&neighbor_pos) = placed_positions.get(&neighbor) {
            // Get neighbor's bounds to place adjacent
            let neighbor_bounds = group_local_bounds.get(&neighbor).copied().unwrap_or(RectI {
                x: 0, y: 0, w: cfg.min_group_size.w, h: cfg.min_group_size.h,
            });

            // Generate positions around the neighbor group (right, below, left, above)
            let positions = [
                // Right of neighbor
                PointI { 
                    x: neighbor_pos.x + neighbor_bounds.w + cfg.gap, 
                    y: neighbor_pos.y 
                },
                // Below neighbor
                PointI { 
                    x: neighbor_pos.x, 
                    y: neighbor_pos.y + neighbor_bounds.h + cfg.gap 
                },
                // Left of neighbor (if there's room)
                PointI { 
                    x: (neighbor_pos.x - cfg.min_group_size.w - cfg.gap).max(cfg.group_padding), 
                    y: neighbor_pos.y 
                },
                // Above neighbor (if there's room)
                PointI { 
                    x: neighbor_pos.x, 
                    y: (neighbor_pos.y - cfg.min_group_size.h - cfg.gap).max(cfg.group_padding) 
                },
                // Diagonal positions
                PointI { 
                    x: neighbor_pos.x + neighbor_bounds.w + cfg.gap, 
                    y: neighbor_pos.y + neighbor_bounds.h + cfg.gap 
                },
                PointI { 
                    x: (neighbor_pos.x - cfg.min_group_size.w - cfg.gap).max(cfg.group_padding), 
                    y: neighbor_pos.y + neighbor_bounds.h + cfg.gap 
                },
            ];

            for pos in positions {
                if pos.x >= cfg.group_padding && pos.y >= cfg.group_padding {
                    candidates.push(pos);
                }
            }
        }
    }

    // Add grid fallback positions
    let remaining = NUM_CANDIDATES.saturating_sub(candidates.len());
    if remaining > 0 {
        generate_group_grid_candidates(&mut candidates, cfg, remaining);
    }

    candidates
}

/// Generate grid positions for groups.
fn generate_group_grid_candidates(candidates: &mut Vec<PointI>, cfg: &LayoutConfig, count: usize) {
    let step_x = cfg.min_group_size.w + cfg.gap;
    let step_y = cfg.min_group_size.h + cfg.gap;
    let cols = (cfg.max_row_w / step_x).max(1);

    for i in 0..count {
        let col = i as i32 % cols;
        let row = i as i32 / cols;
        candidates.push(PointI {
            x: cfg.group_padding + col * step_x,
            y: cfg.group_padding + row * step_y,
        });
    }
}

/// Find the best position for a group (lowest edge length score, non-overlapping).
fn find_best_group_position(
    candidates: &[PointI],
    size: SizeI,
    spatial: &SpatialGrid,
    gid: GroupId,
    placed_positions: &HashMap<GroupId, PointI>,
    group_neighbors: &HashMap<GroupId, Vec<(GroupId, usize)>>,
    group_local_bounds: &HashMap<GroupId, RectI>,
    cfg: &LayoutConfig,
) -> PointI {
    let mut best_pos = PointI { x: cfg.group_padding, y: cfg.group_padding };
    let mut best_score = i64::MAX;

    for &pos in candidates {
        let rect = RectI { x: pos.x, y: pos.y, w: size.w, h: size.h };

        // Skip overlapping positions
        if spatial.overlaps_any(&rect) {
            continue;
        }

        // Score = sum of squared distances between group centers
        let mut score: i64 = 0;
        let center_x = pos.x + size.w / 2;
        let center_y = pos.y + size.h / 2;

        if let Some(neighbors) = group_neighbors.get(&gid) {
            for (neighbor, weight) in neighbors {
                if let Some(&neighbor_pos) = placed_positions.get(neighbor) {
                    let neighbor_bounds = group_local_bounds.get(neighbor).copied().unwrap_or(RectI {
                        x: 0, y: 0, w: cfg.min_group_size.w, h: cfg.min_group_size.h,
                    });
                    let neighbor_cx = neighbor_pos.x + neighbor_bounds.w / 2;
                    let neighbor_cy = neighbor_pos.y + neighbor_bounds.h / 2;

                    let dx = (center_x - neighbor_cx) as i64;
                    let dy = (center_y - neighbor_cy) as i64;
                    // Weighted squared distance
                    score += (dx * dx + dy * dy) * (*weight as i64);
                }
            }
        }

        if score < best_score {
            best_score = score;
            best_pos = pos;
        }
    }

    // Fallback if all candidates overlap
    if best_score == i64::MAX {
        best_pos = find_first_free_position(cfg, spatial, size);
    }

    best_pos
}

/// Optimize node positions by trying to move them closer to their neighbors.
fn optimize_node_positions(
    free_classes: &[ClassId],
    class_local_pos: &mut HashMap<ClassId, PointI>,
    cfg: &LayoutConfig,
    adjacency: &Adjacency,
    _fixed_classes: &[ClassId],
    _fixed_groups: &[GroupId],
    _group_local_bounds: &HashMap<GroupId, RectI>,
) {
    let mut changed = true;
    let mut iter = 0;
    let max_iters = 5;

    while changed && iter < max_iters {
        changed = false;
        iter += 1;

        // Simplified: just add ALL items currently in their positions to spatial grid
        let mut current_rects: Vec<(ClassId, RectI)> = Vec::new();
        for (cid, pos) in class_local_pos.iter() {
             current_rects.push((*cid, RectI { x: pos.x, y: pos.y, w: cfg.class_size.w, h: cfg.class_size.h }));
        }

        // Add fixed groups as obstacles
        let _fixed_group_rects: Vec<RectI> = Vec::new();
        // Since we don't have easy access to fixed group positions/bounds here (they are in diagram/group_local_bounds),
        // and fixed items are usually handled by initial placement, we'll skip strict overlap check against groups 
        // for node optimization in this simplified pass. Fixed classes ARE in class_local_pos so they are checked.

        // Sort by id for determinism
        let mut nodes_to_optimize = free_classes.to_vec();
        nodes_to_optimize.sort_by_key(|c| c.0);

        for cid in nodes_to_optimize {
            let current_pos = match class_local_pos.get(&cid) {
                Some(p) => *p,
                None => continue,
            };
            
            // Calculate ideal position (centroid of neighbors)
            let mut sum_x = 0;
            let mut sum_y = 0;
            let mut count = 0;
            
            for (neighbor, _) in adjacency.get_neighbors(cid) {
                if let Some(pos) = class_local_pos.get(neighbor) {
                    sum_x += pos.x;
                    sum_y += pos.y;
                    count += 1;
                }
            }

            if count > 0 {
                let ideal_x = sum_x / count;
                let ideal_y = sum_y / count;

                // Try to move towards ideal, but keep grid alignment
                let step_x = cfg.class_size.w + cfg.gap;
                let step_y = cfg.class_size.h + cfg.gap;

                // Snap ideal to grid relative to padding
                let snap_x = ((ideal_x - cfg.group_padding) as f32 / step_x as f32).round() as i32 * step_x + cfg.group_padding;
                let snap_y = ((ideal_y - cfg.group_padding) as f32 / step_y as f32).round() as i32 * step_y + cfg.group_padding;
                
                let target = PointI { x: snap_x.max(cfg.group_padding), y: snap_y.max(cfg.group_padding) };

                if target != current_pos {
                    // Check if target is free
                    let rect = RectI { x: target.x, y: target.y, w: cfg.class_size.w, h: cfg.class_size.h };
                    
                    // Check vs all OTHER nodes
                    let mut overlap = false;
                    for (other_cid, other_rect) in &current_rects {
                        if *other_cid != cid && other_rect.overlaps(&rect) {
                            overlap = true;
                            break;
                        }
                    }

                    if !overlap {
                        class_local_pos.insert(cid, target);
                        // Update current_rects for subsequent checks in this pass
                        for item in &mut current_rects {
                            if item.0 == cid {
                                item.1 = rect;
                                break;
                            }
                        }
                        changed = true;
                    }
                }
            }
        }
    }
}

/// Optimize group positions
fn optimize_group_positions(
    free_groups: &[GroupId],
    group_local_pos: &mut HashMap<GroupId, PointI>,
    cfg: &LayoutConfig,
    diagram: &Diagram,
    group_local_bounds: &HashMap<GroupId, RectI>,
    _fixed_classes: &[ClassId],
    _class_local_pos: &HashMap<ClassId, PointI>,
) {
    use super::adjacency::compute_group_adjacency;
    let group_adj = compute_group_adjacency(diagram);
    let mut group_neighbors: HashMap<GroupId, Vec<GroupId>> = HashMap::new();
     for ((g1, g2), _) in group_adj {
        group_neighbors.entry(g1).or_default().push(g2);
        group_neighbors.entry(g2).or_default().push(g1);
    }
    
    // Simple centroid attraction
    let mut changed = true;
    let mut iter = 0;
    while changed && iter < 3 {
        changed = false;
        iter += 1;

        let mut groups_to_optim = free_groups.to_vec();
        groups_to_optim.sort_by_key(|g| g.0);
        
        for gid in groups_to_optim {
            let current_pos = match group_local_pos.get(&gid) {
                Some(p) => *p,
                None => continue,
            };
            let bounds = match group_local_bounds.get(&gid) {
                Some(b) => b,
                None => continue,
            };
            let size = SizeI { w: bounds.w, h: bounds.h };

             // Centroid of connected groups
            let mut sum_x = 0;
            let mut sum_y = 0;
            let mut count = 0;

            if let Some(neighbors) = group_neighbors.get(&gid) {
                for neighbor in neighbors {
                    if let Some(pos) = group_local_pos.get(neighbor) {
                        sum_x += pos.x;
                        sum_y += pos.y;
                        count += 1;
                    }
                }
            }

            if count > 0 {
                let ideal_x = sum_x / count;
                let ideal_y = sum_y / count;
                
                 // Try to move closer
                let target = PointI { x: ideal_x.max(cfg.group_padding), y: ideal_y.max(cfg.group_padding) };
                
                if (target.x - current_pos.x).abs() > 10 || (target.y - current_pos.y).abs() > 10 {
                     // Check overlap
                    let rect = RectI { x: target.x, y: target.y, w: size.w, h: size.h };
                    let mut overlap = false;
                     // Check vs all other groups
                    for (&other_gid, &other_pos) in group_local_pos.iter() {
                        if other_gid != gid {
                            if let Some(other_bounds) = group_local_bounds.get(&other_gid) {
                                let other_rect = RectI { x: other_pos.x, y: other_pos.y, w: other_bounds.w, h: other_bounds.h };
                                if rect.overlaps(&other_rect) {
                                    overlap = true;
                                    break;
                                }
                            }
                        }
                    }

                    if !overlap {
                        group_local_pos.insert(gid, target);
                        changed = true;
                    }
                }
            }
        }
    }
}
