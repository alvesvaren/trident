// Layout module for Trident diagrams.
//
// This module provides layout algorithms for positioning diagram elements.
// Available layout algorithms:
// - "hierarchical" (default): Graph-driven layout that places connected nodes closer together
// - "grid": Simple left-to-right, top-to-bottom grid layout
//
// Submodules:
// - spatial_grid: O(1) overlap detection
// - adjacency: Edge weight computation
// - placement: Graph-driven placement algorithm
// - graph_driven: Default hierarchical layout
// - grid: Simple grid layout

use std::collections::HashMap;

use crate::parser::{PointI, Diagram, GroupId, NodeId};
use serde::Serialize;

mod spatial_grid;
pub mod adjacency;
pub mod placement;
pub mod algorithms;

pub use algorithms::{GraphDrivenLayout, layout_graph_driven, GridLayout, layout_grid};


#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub struct SizeI {
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
pub struct RectI {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl RectI {
    pub fn right(&self) -> i32 { self.x + self.w }
    pub fn bottom(&self) -> i32 { self.y + self.h }

    pub fn overlaps(&self, other: &RectI) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    pub fn union(&self, other: &RectI) -> RectI {
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = self.right().max(other.right());
        let y1 = self.bottom().max(other.bottom());
        RectI { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }
}

#[derive(Debug, Clone)]
pub struct LayoutConfig {
    /// Padding inside groups.
    pub group_padding: i32,
    /// Spacing between siblings during packing.
    pub gap: i32,
    /// Max row width before wrapping. Small graphs can ignore.
    pub max_row_w: i32,
    /// Size for nodes.
    pub node_size: SizeI,
    /// Minimum size for groups (even if empty).
    pub min_group_size: SizeI,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            group_padding: 24,
            gap: 24,
            max_row_w: 1000,
            node_size: SizeI { w: 220, h: 120 },
            min_group_size: SizeI { w: 200, h: 120 },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutResult {
    /// Local positions (relative to parent group) for all groups/nodes.
    pub group_local_pos: HashMap<GroupId, PointI>,
    pub node_local_pos: HashMap<NodeId, PointI>,

    /// World positions for all groups/nodes (after accumulation).
    pub group_world_pos: HashMap<GroupId, PointI>,
    pub node_world_pos: HashMap<NodeId, PointI>,

    /// Group bounds in world coordinates (including padding).
    pub group_world_bounds: HashMap<GroupId, RectI>,

    /// Node bounds in world coordinates.
    pub node_world_bounds: HashMap<NodeId, RectI>,
}

// ============================================================================
// Layout Strategy Trait - Dependency Inversion
// ============================================================================

/// Trait for layout strategies - enables dependency inversion.
/// Implement this trait to create custom layout algorithms.
pub trait LayoutStrategy {
    fn layout(&self, diagram: &Diagram, cfg: &LayoutConfig) -> LayoutResult;
}

/// Main entry point - dispatches to the appropriate layout algorithm.
/// 
/// # Arguments
/// * `diagram` - The diagram to layout
/// * `cfg` - Layout configuration
/// * `algorithm` - Layout algorithm name: "hierarchical" (default) or "grid"
pub fn layout_diagram(diagram: &Diagram, cfg: &LayoutConfig, algorithm: &str) -> LayoutResult {
    match algorithm {
        "grid" => layout_grid(diagram, cfg),
        "hierarchical" | _ => layout_graph_driven(diagram, cfg),
    }
}

/// Layout with a custom strategy.
pub fn layout_diagram_with_strategy<S: LayoutStrategy>(
    diagram: &Diagram,
    cfg: &LayoutConfig,
    strategy: &S,
) -> LayoutResult {
    strategy.layout(diagram, cfg)
}

// ============================================================================
// Shared Utilities
// ============================================================================

/// Compute a group's container bounds in LOCAL coordinates, based on children.
/// This includes padding. If group has no children, returns min_group_size at (0,0).
pub fn compute_group_local_bounds(
    diagram: &Diagram,
    gid: GroupId,
    cfg: &LayoutConfig,
    group_local_pos: &HashMap<GroupId, PointI>,
    node_local_pos: &HashMap<NodeId, PointI>,
    group_local_bounds: &HashMap<GroupId, RectI>,
) -> RectI {
    let g = &diagram.groups[gid.0];

    let mut any = false;
    let mut bb: RectI = RectI { x: 0, y: 0, w: 0, h: 0 };

    // include child groups
    for &cgid in &g.children_groups {
        let p = *group_local_pos.get(&cgid).unwrap_or(&PointI { x: 0, y: 0 });
        let lb = group_local_bounds.get(&cgid).copied().unwrap_or(RectI {
            x: 0,
            y: 0,
            w: cfg.min_group_size.w,
            h: cfg.min_group_size.h,
        });
        let r = RectI { x: p.x, y: p.y, w: lb.w, h: lb.h };
        bb = if any { bb.union(&r) } else { any = true; r };
    }

    // include child nodes
    for &nid in &g.children_nodes {
        let p = *node_local_pos.get(&nid).unwrap_or(&PointI { x: 0, y: 0 });
        let sz = cfg.node_size;
        let r = RectI { x: p.x, y: p.y, w: sz.w, h: sz.h };
        bb = if any { bb.union(&r) } else { any = true; r };
    }

    if !any {
        // Empty group box
        return RectI {
            x: 0,
            y: 0,
            w: cfg.min_group_size.w,
            h: cfg.min_group_size.h,
        };
    }

    // Expand by padding on all sides
    RectI {
        x: bb.x - cfg.group_padding,
        y: bb.y - cfg.group_padding,
        w: bb.w + 2 * cfg.group_padding,
        h: bb.h + 2 * cfg.group_padding,
    }
}

/// Post-order group traversal: children before parent.
/// Deterministic order: respects diagram.groups[gid].children_groups order.
pub fn post_order_groups(diagram: &Diagram) -> Vec<GroupId> {
    fn dfs(diagram: &Diagram, gid: GroupId, out: &mut Vec<GroupId>) {
        let g = &diagram.groups[gid.0];
        for &c in &g.children_groups {
            dfs(diagram, c, out);
        }
        out.push(gid);
    }
    let mut out = Vec::new();
    dfs(diagram, diagram.root, &mut out);
    out
}

/// Pre-order group traversal: parent before children.
pub fn pre_order_groups(diagram: &Diagram) -> Vec<GroupId> {
    fn dfs(diagram: &Diagram, gid: GroupId, out: &mut Vec<GroupId>) {
        out.push(gid);
        let g = &diagram.groups[gid.0];
        for &c in &g.children_groups {
            dfs(diagram, c, out);
        }
    }
    let mut out = Vec::new();
    dfs(diagram, diagram.root, &mut out);
    out
}
