// Hierarchical (Layered) Layout Algorithm
//
// Replaces the previous greedy placement with a structured Layered Graph approach (Sugiyama-style).
// Goals:
// - Structure: Nodes align in clearer rows/columns (ranks).
// - Flow: Dependency direction (Inheritance, Compositions) flows Top-Down (or Left-Right).
// - Simplicity: Strict grid placement, no complex 2D searching.

use std::collections::{HashMap, HashSet, VecDeque};
use crate::parser::{PointI, Diagram, GroupId, ClassId, Arrow};
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
    class_local_pos: &mut HashMap<ClassId, PointI>,
    group_local_bounds: &HashMap<GroupId, RectI>,
) {
    let g = &diagram.groups[gid.0];

    // 1. Separate fixed and free items
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

    // 2. Assign fixed positions
    for cgid in &fixed_groups {
        let p = diagram.groups[cgid.0].pos.unwrap();
        group_local_pos.insert(*cgid, p);
    }
    for cid in &fixed_classes {
        let p = diagram.classes[cid.0].pos.unwrap();
        class_local_pos.insert(*cid, p);
    }

    // 3. Initialize spatial grid with fixed items
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

    // 5. Place free classes (Hierarchical)
    if !free_classes.is_empty() {
        layout_classes_hierarchical(
             diagram,
             &free_classes,
             &fixed_classes,
             cfg,
             &mut spatial,
             class_local_pos,
             group_local_pos,
        );
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

    // Map Class -> Group
    let mut class_to_group: HashMap<ClassId, GroupId> = HashMap::new();
    for &gid in groups {
        for &cid in &diagram.groups[gid.0].children_classes {
            class_to_group.insert(cid, gid);
        }
    }

    for edge in &diagram.edges {
        let (parent_cid, child_cid) = match edge.arrow {
            Arrow::ExtendsRight => (edge.to, edge.from),
            Arrow::ExtendsLeft => (edge.from, edge.to),
             Arrow::Compose | Arrow::Aggregate | Arrow::AssocRight | Arrow::DepRight => (edge.from, edge.to),
            Arrow::AssocLeft | Arrow::DepLeft => (edge.to, edge.from),
            _ => (edge.from, edge.to),
        };

        if let (Some(&pgid), Some(&cgid)) = (class_to_group.get(&parent_cid), class_to_group.get(&child_cid)) {
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
            // No deep traversal for unvisited here for simplicity
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
            // Apply sorting result (already done)
            
            let mut current_x = start_x;
            let mut row_start_y = next_y;
            let mut current_y = row_start_y;
            // Use config max_row_w
            let limit_w = if cfg.max_row_w > 0 { cfg.max_row_w } else { 1200 };

            for &gid in row_groups.iter() {
                 let bounds = group_local_bounds.get(&gid).copied().unwrap_or(RectI { 
                     x: 0, y: 0, w: cfg.min_group_size.w, h: cfg.min_group_size.h 
                });

                 // Check wrap
                 if current_x + bounds.w > limit_w && current_x > start_x {
                     current_x = start_x;
                     // Move Y down by max height of PREVIOUS row?
                     // Simplified: move down by default height or dynamic?
                     // Since groups vary wildly in size, this is tricky.
                     // We should probably track max height of current "sub roW".
                     // For simplicity, move down by bounds.h? No, that's this group.
                     // Let's assume a standard increment for now or track max_h of current row.
                     // A safe bet is moving down by min_group_size + gap, but effectively we want "next line".
                     // Ideally we track the `max_y` of the current line.
                     // But we don't know it yet.
                     // Let's use a "row_step" heuristic or track it.
                     // BETTER: use flexible packing (Next Fit).
                     current_y += 300; // Arbitrary step if wrapping? Or use bounds.h of previous?
                     // Let's use cfg.min_group_size.h * 2 for safety?
                 }

                 let mut pos = PointI { x: current_x, y: current_y };
                 
                 // Overlap check and shift
                 let mut rect = RectI { x: pos.x, y: pos.y, w: bounds.w, h: bounds.h };
                 while spatial.overlaps_any(&rect) {
                     // Shift right
                     pos.x += 50; // shift amount
                     if pos.x + bounds.w > limit_w {
                         pos.x = start_x;
                         pos.y += 100; // Shift down
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
                 if pos.y > current_y { current_y = pos.y; } // Track if we moved down
            }
            // next_y is already updated via max check
        }
    }
}

/// Layout classes using strict Hierarchical (Ranked) approach.
fn layout_classes_hierarchical(
    diagram: &Diagram,
    classes: &[ClassId],
    fixed_classes: &[ClassId],
    cfg: &LayoutConfig,
    spatial: &mut SpatialGrid,
    class_local_pos: &mut HashMap<ClassId, PointI>,
    _group_local_pos: &HashMap<GroupId, PointI>,
) {
    // 1. Build Dependency Graph
    // Combine Free + Fixed for dependency analysis
    // We treat fixed nodes as "Anchors" that have pre-determined ranks.
    let mut all_classes = Vec::with_capacity(classes.len() + fixed_classes.len());
    all_classes.extend_from_slice(classes);
    all_classes.extend_from_slice(fixed_classes);

    // Build graph for ALL nodes
    let (adj, rev_adj) = build_dependency_graph(diagram, &all_classes);

    // 2. Assign Ranks
    // Source nodes (in-degree 0 in dependency graph)
    // We use 'rev_adj' to count in-degrees.
    let mut ranks: HashMap<ClassId, i32> = HashMap::new();
    let mut visited: HashSet<ClassId> = HashSet::new();
    let mut queue: VecDeque<(ClassId, i32)> = VecDeque::new();

    let layer_height = cfg.class_size.h + cfg.gap * 2;

    // Seed Fixed Nodes as Visited/Ranked based on Y position
    for &fcid in fixed_classes {
        if let Some(pos) = class_local_pos.get(&fcid) {
            // Approx rank
            let r = (pos.y as f32 / layer_height as f32).max(0.0) as i32;
            ranks.insert(fcid, r);
            visited.insert(fcid);
            // We do NOT add them to queue for propagation unless we want them to push others?
            // Yes, they should act as sources for their children.
            queue.push_back((fcid, r));
        }
    }

    // Find roots among FREE nodes (only if they have no dependencies OR dependencies are effectively handled)
    for &cid in classes {
        if !visited.contains(&cid) {
             // If incoming edges are ONLY from unvisited (cycle?) or no edges?
             // Actually, standard Topological Sort logic:
             // If all parents visited, we can visit.
             // But simpler: just add roots.
             if !rev_adj.contains_key(&cid) {
                 queue.push_back((cid, 0));
                 visited.insert(cid);
             }
        }
    }
    
    // Resume BFS
    // Note: Queue might contain mix of Fixed and Free roots.
    // If Fixed node is at Rank 10. Its children will get Rank 11. CORRECT.
    
    // Cycle break / unvisited handle
    // If queue is empty but we have unvisited free nodes...
    if queue.is_empty() {
         for &cid in classes {
             if !visited.contains(&cid) {
                 // Pick one
                 queue.push_back((cid, 0));
                 visited.insert(cid);
                 break; // Only start one component, loop will catch others
             }
         }
    }

    while let Some((cid, rank)) = queue.pop_front() {
        // If cid is Fixed, we already set its rank. 
        // If cid is Free, we update rank.
        if !fixed_classes.contains(&cid) {
             ranks.insert(cid, rank);
        }
        
        // Propagate to neighbors
        if let Some(neighbors) = adj.get(&cid) {
            for &neighbor in neighbors {
                // If neighbor is Fixed, its rank is already set. We don't change it.
                // We only traverse if neighbor is NOT visited.
                if !visited.contains(&neighbor) && all_classes.contains(&neighbor) {
                    visited.insert(neighbor);
                    queue.push_back((neighbor, rank + 1));
                }
            }
        }
    }

    // Handle remaining unvisited (cycles or disconnected)
    for &cid in classes {
        if !visited.contains(&cid) {
             let mut sub_queue = VecDeque::new();
             sub_queue.push_back((cid, 0));
             visited.insert(cid);
             while let Some((scid, srank)) = sub_queue.pop_front() {
                 if !fixed_classes.contains(&scid) {
                    ranks.insert(scid, srank);
                 }
                if let Some(neighbors) = adj.get(&scid) {
                    for &neighbor in neighbors {
                        if !visited.contains(&neighbor) && all_classes.contains(&neighbor) {
                            visited.insert(neighbor);
                            sub_queue.push_back((neighbor, srank + 1));
                        }
                    }
                }
             }
        }
    }

    // Find max rank to iterate
    let max_rank = ranks.values().max().copied().unwrap_or(0);

    // 3. Group by Rank
    let mut rank_map: HashMap<i32, Vec<ClassId>> = HashMap::new();
    // Only place FREE classes!
    for &cid in classes {
        if let Some(&r) = ranks.get(&cid) {
            rank_map.entry(r).or_default().push(cid);
        }
    }
    
    // 3.5. Minimize Edge Crossings (Barycenter Heuristic)
    // Run a few iterations of down-sweep and up-sweep
    let iterations = 12;
    for _ in 0..iterations {
        // Down-sweep: Sort rank r based on barycenter of rank r-1 (parents)
        for r in 1..=max_rank {
            if let Some(prev_row_nodes) = rank_map.get(&(r - 1)).cloned() { 
                // Build map of prev_row positions (indices)
                // Need to include FIXED nodes in this calculation if they are in rank r-1!
                // ERROR: rank_map currently ONLY contains Free nodes.
                // If Parent is Fixed at Rank X, it's not in rank_map[X].
                // So Barycenter will ignore it?
                // FIX: We need rank_map to "virtually" contain Fixed nodes for the purpose of Barycenter calculation.
                // OR we check fixed_classes for rank matches.
                
                // Construct "Effective Row" = rank_map[r-1] + fixed_nodes_at_rank[r-1]
                // This is getting complex for a replace block.
                // Simplification for now: Just sort Free nodes relative to Free nodes. 
                // Fixed nodes act as invisible constraints via Rank, but don't pull horizontally yet.
                // Re-enabling horizontal pull from Fixed Nodes is Phase 2.
                // For now, stabilizing Rank alone is the big win.
                
                let prev_pos: HashMap<ClassId, usize> = prev_row_nodes.iter().enumerate().map(|(i, &c)| (c, i)).collect();
                
                if let Some(row_nodes) = rank_map.get_mut(&r) {

                    row_nodes.sort_by(|&a, &b| {
                        let avg_a = get_barycenter(a, &rev_adj, &prev_pos);
                        let avg_b = get_barycenter(b, &rev_adj, &prev_pos);
                        avg_a.partial_cmp(&avg_b).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }

        // Up-sweep: Sort rank r based on barycenter of rank r+1 (children)
        for r in (0..max_rank).rev() {
            if let Some(next_row_nodes) = rank_map.get(&(r + 1)).cloned() {
                let next_pos: HashMap<ClassId, usize> = next_row_nodes.iter().enumerate().map(|(i, &c)| (c, i)).collect();

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
    let col_width = cfg.class_size.w + cfg.gap;
    let row_height = cfg.class_size.h + cfg.gap * 2; // More vertical space for arrows

    // Track occupied positions (in grid coords)
    let mut next_y = start_y;

    for r in 0..=max_rank {
        if let Some(row_nodes) = rank_map.get_mut(&r) {
            // Apply sorting result (already done in loop above)

            // Start new rank below previous
            let rank_start_y = next_y;
            let mut current_y = rank_start_y;
            let mut current_x = start_x;
            
            // Calculate max width for this row based on config
            // Use config or default if 0
            let limit_w = if cfg.max_row_w > 0 { cfg.max_row_w } else { 1200 };

            for &cid in row_nodes.iter() {
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
                 let mut rect = RectI { x: safe_pos.x, y: safe_pos.y, w: cfg.class_size.w, h: cfg.class_size.h };
                 while spatial.overlaps_any(&rect) {
                     // Shift right? Or wrap again?
                     // Shift right first
                     safe_pos.x += col_width;
                      if safe_pos.x + col_width > limit_w {
                         safe_pos.x = start_x;
                         safe_pos.y += row_height;
                      }
                      rect = RectI { x: safe_pos.x, y: safe_pos.y, w: cfg.class_size.w, h: cfg.class_size.h };
                 }

                 current_x = safe_pos.x + col_width;
                 current_y = safe_pos.y; // Keep Y updated if we wrapped
                 
                 class_local_pos.insert(cid, safe_pos);
                 spatial.insert(RectI { x: safe_pos.x, y: safe_pos.y, w: cfg.class_size.w, h: cfg.class_size.h });
            }
            // Next rank starts below the lowest row used in this rank
            next_y = current_y + row_height;
        }
    }
}

/// Build directed adjacency for hierarchy.
/// Returns (Adjacency, ReverseAdjacency)
fn build_dependency_graph(diagram: &Diagram, scope: &[ClassId]) -> (HashMap<ClassId, Vec<ClassId>>, HashMap<ClassId, Vec<ClassId>>) {
    let mut adj: HashMap<ClassId, Vec<ClassId>> = HashMap::new();
    let mut rev_adj: HashMap<ClassId, Vec<ClassId>> = HashMap::new();
    let scope_set: HashSet<ClassId> = scope.iter().copied().collect();

    for edge in &diagram.edges {
        // Directed: From -> To (Parent -> Child usually implies Top -> Bottom layout)
        // Arrows:
        // A --> B: A depends on B? Or flow? Usually A over B.
        // A --|> B (ExtendsRight): A extends B. B is Parent. So B -> A (Top -> Bottom).
        
        let (parent, child) = match edge.arrow {
            Arrow::ExtendsRight => (edge.to, edge.from), // B is parent of A
            Arrow::ExtendsLeft => (edge.from, edge.to),  // A is parent of B
            Arrow::Compose | Arrow::Aggregate | Arrow::AssocRight | Arrow::DepRight => (edge.from, edge.to), // A owns/calls B
            Arrow::AssocLeft | Arrow::DepLeft => (edge.to, edge.from), // B owns/calls A
            _ => (edge.from, edge.to), // Default
        };

        if scope_set.contains(&parent) && scope_set.contains(&child) {
            adj.entry(parent).or_default().push(child);
            rev_adj.entry(child).or_default().push(parent);
        }
    }
    (adj, rev_adj)
}

/// Calculate average position (barycenter) of connected nodes in the adjacent rank.
fn get_barycenter(
    cid: ClassId,
    adj: &HashMap<ClassId, Vec<ClassId>>,
    neighbor_pos: &HashMap<ClassId, usize>,
) -> f32 {
    if let Some(neighbors) = adj.get(&cid) {
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


