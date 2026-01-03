//! Simple grid layout algorithm.
//!
//! This is a straightforward layout that places nodes in a simple grid pattern,
//! left-to-right, top-to-bottom. It ignores edge relationships and just packs
//! nodes efficiently.
//!
//! This is intended as a simple, easy-to-understand alternative to the
//! hierarchical layout, useful for understanding the layout system.

use std::collections::HashMap;
use crate::parser::{PointI, Diagram, GroupId, NodeId};
use crate::layout::{RectI, LayoutConfig, LayoutResult, LayoutStrategy};
use crate::layout::{post_order_groups, pre_order_groups, compute_group_local_bounds};

/// Simple grid layout implementation.
/// Places nodes in a left-to-right, top-to-bottom grid pattern.
pub struct GridLayout;

impl LayoutStrategy for GridLayout {
    fn layout(&self, diagram: &Diagram, cfg: &LayoutConfig) -> LayoutResult {
        layout_grid(diagram, cfg)
    }
}

/// Layout nodes in a simple grid pattern.
pub fn layout_grid(diagram: &Diagram, cfg: &LayoutConfig) -> LayoutResult {
    let mut group_local_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut node_local_pos: HashMap<NodeId, PointI> = HashMap::new();

    group_local_pos.insert(diagram.root, PointI { x: 0, y: 0 });

    // Layout groups bottom-up (children first)
    let post = post_order_groups(diagram);
    let mut group_local_bounds: HashMap<GroupId, RectI> = HashMap::new();

    for gid in post {
        let g = &diagram.groups[gid.0];

        // Set group position
        if gid != diagram.root {
            let p = g.pos.unwrap_or(PointI { x: 0, y: 0 });
            group_local_pos.insert(gid, p);
        }

        // Layout children in this group using simple grid
        layout_group_children_grid(
            diagram,
            gid,
            cfg,
            &mut group_local_pos,
            &mut node_local_pos,
            &group_local_bounds,
        );

        // Compute this group's local bounds
        let bounds = compute_group_local_bounds(
            diagram,
            gid,
            cfg,
            &group_local_pos,
            &node_local_pos,
            &group_local_bounds,
        );
        group_local_bounds.insert(gid, bounds);
    }

    // Second pass: accumulate world positions
    let mut group_world_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut node_world_pos: HashMap<NodeId, PointI> = HashMap::new();
    let mut group_world_bounds: HashMap<GroupId, RectI> = HashMap::new();
    let mut node_world_bounds: HashMap<NodeId, RectI> = HashMap::new();

    group_world_pos.insert(diagram.root, PointI { x: 0, y: 0 });

    let pre = pre_order_groups(diagram);

    for gid in pre {
        let g_local = *group_local_pos.get(&gid).unwrap_or(&PointI { x: 0, y: 0 });
        let g_world = if gid == diagram.root {
            PointI { x: 0, y: 0 }
        } else {
            let parent = diagram.groups[gid.0].parent.expect("non-root group must have parent");
            let pw = *group_world_pos.get(&parent).unwrap();
            PointI { x: pw.x + g_local.x, y: pw.y + g_local.y }
        };

        group_world_pos.insert(gid, g_world);

        let lb = *group_local_bounds.get(&gid).unwrap();
        let wb = RectI { x: g_world.x + lb.x, y: g_world.y + lb.y, w: lb.w, h: lb.h };
        group_world_bounds.insert(gid, wb);

        for &nid in &diagram.groups[gid.0].children_nodes {
            let n_local = *node_local_pos.get(&nid).unwrap_or(&PointI { x: 0, y: 0 });
            let n_world = PointI { x: g_world.x + n_local.x, y: g_world.y + n_local.y };
            node_world_pos.insert(nid, n_world);

            let node = &diagram.nodes[nid.0];
            let sz = crate::layout::get_node_size(node, cfg);
            node_world_bounds.insert(nid, RectI { x: n_world.x, y: n_world.y, w: sz.w, h: sz.h });
        }
    }

    LayoutResult {
        group_local_pos,
        node_local_pos,
        group_world_pos,
        node_world_pos,
        group_world_bounds,
        node_world_bounds,
    }
}

/// Layout children of a group in a simple grid pattern.
/// Fixed nodes respect their position, other nodes fill in left-to-right, top-to-bottom.
fn layout_group_children_grid(
    diagram: &Diagram,
    gid: GroupId,
    cfg: &LayoutConfig,
    group_local_pos: &mut HashMap<GroupId, PointI>,
    node_local_pos: &mut HashMap<NodeId, PointI>,
    group_local_bounds: &HashMap<GroupId, RectI>,
) {
    let g = &diagram.groups[gid.0];
    let padding = cfg.group_padding;
    let gap = cfg.gap;
    let node_size = cfg.node_size;
    let max_row_w = cfg.max_row_w;

    // Calculate how many nodes fit per row
    let nodes_per_row = ((max_row_w - 2 * padding) / (node_size.w + gap)).max(1) as usize;

    // Separate fixed and auto-placed items
    let mut auto_nodes: Vec<NodeId> = Vec::new();
    let mut auto_groups: Vec<GroupId> = Vec::new();

    // Handle fixed nodes first
    for &nid in &g.children_nodes {
        let node = &diagram.nodes[nid.0];
        if let Some(pos) = node.pos {
            node_local_pos.insert(nid, pos);
        } else {
            auto_nodes.push(nid);
        }
    }

    // Handle fixed groups first
    for &cgid in &g.children_groups {
        let child_g = &diagram.groups[cgid.0];
        if let Some(pos) = child_g.pos {
            group_local_pos.insert(cgid, pos);
        } else {
            auto_groups.push(cgid);
        }
    }

    // Place auto-layout nodes in a grid
    let mut x = padding;
    let mut y = padding;
    let mut col = 0;

    for nid in auto_nodes {
        node_local_pos.insert(nid, PointI { x, y });
        
        col += 1;
        if col >= nodes_per_row {
            col = 0;
            x = padding;
            y += node_size.h + gap;
        } else {
            x += node_size.w + gap;
        }
    }

    // Move to next row if we have groups
    if !auto_groups.is_empty() && col > 0 {
        y += node_size.h + gap;
        x = padding;
    }

    // Place auto-layout groups in a row below nodes
    for cgid in auto_groups {
        let bounds = group_local_bounds.get(&cgid).copied().unwrap_or(RectI {
            x: 0,
            y: 0,
            w: cfg.min_group_size.w,
            h: cfg.min_group_size.h,
        });

        // Check if group fits on current row
        if x + bounds.w > max_row_w && x > padding {
            x = padding;
            y += bounds.h + gap;
        }

        group_local_pos.insert(cgid, PointI { x, y });
        x += bounds.w + gap;
    }
}
