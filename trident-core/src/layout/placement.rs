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

    // Place free classes using graph-driven algorithm
    let mut pending_classes: Vec<ClassId> = free_classes;

    while !pending_classes.is_empty() {
        // Pick next node: highest edge weight to placed nodes, tie-break by order
        let next_idx = pick_next_class(
            &pending_classes,
            &placed_classes,
            adjacency,
            diagram,
        );
        let next_cid = pending_classes.remove(next_idx);

        let sz = cfg.class_size;
        let weights = node_weights.get(&next_cid);

        // Generate candidate positions
        let candidates = generate_candidates(
            next_cid,
            &class_positions,
            adjacency,
            cfg,
            &placed_classes,
        );

        // Find best non-overlapping candidate
        let pos = find_best_position(
            &candidates,
            sz,
            &spatial,
            next_cid,
            &class_positions,
            adjacency,
            weights,
            cfg,
        );

        // Place the node
        class_local_pos.insert(next_cid, pos);
        class_positions.insert(next_cid, pos);
        placed_classes.insert(next_cid);
        spatial.insert(RectI { x: pos.x, y: pos.y, w: sz.w, h: sz.h });
    }

    // Place free groups using similar approach
    // For now, use simpler group placement based on inter-group edges
    place_free_groups(
        diagram,
        &free_groups,
        cfg,
        &mut spatial,
        group_local_pos,
        &mut placed_groups,
        group_local_bounds,
    );
}

/// Pick the next class to place based on connectivity to already-placed nodes.
fn pick_next_class(
    pending: &[ClassId],
    placed: &HashSet<ClassId>,
    adjacency: &Adjacency,
    diagram: &Diagram,
) -> usize {
    let mut best_idx = 0;
    let mut best_score = 0usize;
    let mut best_order = usize::MAX;

    for (idx, &cid) in pending.iter().enumerate() {
        // Score = sum of edge weights to placed nodes
        let mut score = 0usize;
        for (neighbor, weight) in adjacency.get_neighbors(cid) {
            if placed.contains(neighbor) {
                score += weight;
            }
        }

        // If no placed neighbors, use total degree
        if score == 0 && placed.is_empty() {
            score = adjacency.get_degree(cid);
        }

        let order = diagram.classes[cid.0].order;

        // Higher score wins, tie-break by earlier order
        if score > best_score || (score == best_score && order < best_order) {
            best_idx = idx;
            best_score = score;
            best_order = order;
        }
    }

    best_idx
}

/// Generate candidate positions around connected placed nodes.
fn generate_candidates(
    cid: ClassId,
    placed_positions: &HashMap<ClassId, PointI>,
    adjacency: &Adjacency,
    cfg: &LayoutConfig,
    placed: &HashSet<ClassId>,
) -> Vec<PointI> {
    let mut candidates = Vec::with_capacity(NUM_CANDIDATES);

    // Find strongest connected placed neighbor
    let mut best_neighbor: Option<ClassId> = None;
    let mut best_weight = 0usize;

    for (neighbor, weight) in adjacency.get_neighbors(cid) {
        if placed.contains(neighbor) && *weight > best_weight {
            best_neighbor = Some(*neighbor);
            best_weight = *weight;
        }
    }

    if let Some(neighbor) = best_neighbor {
        if let Some(&center) = placed_positions.get(&neighbor) {
            // Generate positions in a spiral around the neighbor
            let step = cfg.class_size.w + cfg.gap;
            generate_spiral_candidates(&mut candidates, center, step, cfg);
        }
    }

    // If no placed neighbors or not enough candidates, add grid positions
    if candidates.len() < NUM_CANDIDATES {
        let remaining = NUM_CANDIDATES - candidates.len();
        generate_grid_candidates(&mut candidates, cfg, remaining);
    }

    candidates
}

/// Generate positions in a spiral pattern around a center point.
fn generate_spiral_candidates(
    candidates: &mut Vec<PointI>,
    center: PointI,
    step: i32,
    cfg: &LayoutConfig,
) {
    // 8 directions around center (right, down, left, up, and diagonals)
    let directions: [(i32, i32); 8] = [
        (1, 0), (1, 1), (0, 1), (-1, 1),
        (-1, 0), (-1, -1), (0, -1), (1, -1),
    ];

    // Multiple rings
    for ring in 1..=4 {
        for &(dx, dy) in &directions {
            let x = center.x + dx * step * ring;
            let y = center.y + dy * step * ring;

            // Clamp to valid area (at least at padding)
            if x >= cfg.group_padding && y >= cfg.group_padding {
                candidates.push(PointI { x, y });
            }

            if candidates.len() >= NUM_CANDIDATES {
                return;
            }
        }
    }
}

/// Generate grid positions (fallback for nodes with no placed neighbors).
fn generate_grid_candidates(candidates: &mut Vec<PointI>, cfg: &LayoutConfig, count: usize) {
    let step_x = cfg.class_size.w + cfg.gap;
    let step_y = cfg.class_size.h + cfg.gap;
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

/// Place free groups (simpler algorithm for now).
fn place_free_groups(
    _diagram: &Diagram,
    free_groups: &[GroupId],
    cfg: &LayoutConfig,
    spatial: &mut SpatialGrid,
    group_local_pos: &mut HashMap<GroupId, PointI>,
    placed_groups: &mut HashSet<GroupId>,
    group_local_bounds: &HashMap<GroupId, RectI>,
) {
    // For now, place groups in order using simple row-packing
    // TODO: Use inter-group edge weights for smarter placement
    
    for &gid in free_groups {
        let lb = group_local_bounds.get(&gid).copied().unwrap_or(RectI {
            x: 0, y: 0, w: cfg.min_group_size.w, h: cfg.min_group_size.h,
        });
        let size = SizeI { w: lb.w, h: lb.h };

        let pos = find_first_free_position(cfg, spatial, size);

        group_local_pos.insert(gid, pos);
        placed_groups.insert(gid);
        spatial.insert(RectI { x: pos.x, y: pos.y, w: size.w, h: size.h });
    }
}
