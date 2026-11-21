// Layout module for Trident diagrams.
//
// This module provides layout algorithms for positioning diagram elements.
// Available layout algorithms:
// - "organic" (default): Stress-based layout with crossing reduction
// - "hierarchical": Layered layout that places connected nodes in rows
// - "grid": Simple left-to-right, top-to-bottom grid layout
//
// Submodules:
// - spatial_grid: O(1) overlap detection
// - adjacency: Edge weight computation
// - placement: Graph-driven placement algorithm
// - graph_driven: Hierarchical layout
// - grid: Simple grid layout
// - organic: Default organic layout

use std::collections::HashMap;

use crate::parser::{PointI, Diagram, GroupId, NodeId};
use serde::Serialize;

mod spatial_grid;
pub mod adjacency;
pub mod placement;
pub mod algorithms;

pub use algorithms::{GraphDrivenLayout, layout_graph_driven, GridLayout, layout_grid, OrganicLayout, layout_organic};


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
    /// Size for class-kind nodes (UML boxes).
    pub class_size: SizeI,
    /// Size for node-kind nodes (shapes: rectangle, circle, diamond).
    pub node_size: SizeI,
    /// Minimum size for groups (even if empty).
    pub min_group_size: SizeI,
    /// Rendering constants for content-based sizing (mirrored from SVGNode.tsx)
    pub node_rendering: NodeRenderingConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeRenderingConfig {
    /// Padding around node content
    pub padding: i32,
    /// Height of each line of text
    pub line_height: i32,
    /// Space between lines for separator (---) lines
    pub separator_spacing: i32,
    /// Average character width in monospace font (pixels)
    pub char_width: i32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            group_padding: 24,
            gap: 24,
            max_row_w: 1000,
            class_size: SizeI { w: 220, h: 120 },
            node_size: SizeI { w: 80, h: 80 },
            min_group_size: SizeI { w: 200, h: 120 },
            node_rendering: NodeRenderingConfig {
                padding: 8,
                line_height: 14,
                separator_spacing: 10,
                char_width: 7, // Approximate width of monospace character
            },
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
/// * `algorithm` - Layout algorithm name: "organic" (default), "hierarchical" or "grid"
pub fn layout_diagram(diagram: &Diagram, cfg: &LayoutConfig, algorithm: &str) -> LayoutResult {
    match algorithm {
        "grid" => layout_grid(diagram, cfg),
        "hierarchical" => layout_graph_driven(diagram, cfg),
        _ => layout_organic(diagram, cfg),
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

use crate::parser::compile::Node;

/// Get the size for a node, considering kind, custom dimensions, and config defaults.
/// For unconstrained nodes (no explicit width/height), calculates size based on content.
pub fn get_node_size(node: &Node, cfg: &LayoutConfig) -> SizeI {
    let default = if node.kind == "node" {
        cfg.node_size
    } else {
        cfg.class_size
    };

    let width = match node.width {
        Some(w) => w,
        None => calculate_content_width(node, cfg, default.w),
    };

    let height = match node.height {
        Some(h) => h,
        None => calculate_content_height(node, &cfg.node_rendering),
    };

    SizeI { w: width, h: height }
}

/// Calculate width based on content for unconstrained nodes
fn calculate_content_width(node: &Node, cfg: &LayoutConfig, min_width: i32) -> i32 {
    // For shape nodes ("node" kind), maintain square aspect ratio
    if node.kind == "node" {
        let height = calculate_content_height(node, &cfg.node_rendering);
        return height; // Make width equal to height for squares
    }

    // For class nodes, use minimum width for now
    min_width
}

/// Calculate height based on content for unconstrained nodes
fn calculate_content_height(node: &Node, r: &NodeRenderingConfig) -> i32 {
    let mut num_lines = 0;

    // Count lines
    if !node.modifiers.is_empty() || node.kind != "class" {
        num_lines += 1; // stereotype
    }
    num_lines += 1; // title
    num_lines += 1; // separator
    num_lines += node.body_lines.len(); // body lines

    // Height = padding + (num_lines * line_height) + padding
    r.padding + (num_lines as i32 * r.line_height) + r.padding
}

/// Format modifiers and kind for stereotype display (mirrored from SVGNode.tsx)
fn format_modifiers(modifiers: &[String], kind: &str) -> String {
    let mut parts = Vec::new();

    // Add modifiers as stereotypes
    for modifier in modifiers {
        parts.push(format!("«{}»", modifier));
    }

    // Add kind as stereotype if not "class"
    if kind != "class" {
        parts.push(format!("«{}»", kind));
    }

    parts.join(" ")
}

/// Check if a line is a separator (---)
fn is_separator_line(line: &str) -> bool {
    line.trim().chars().all(|c| c == '-')
}

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
        let node = &diagram.nodes[nid.0];
        let sz = get_node_size(node, cfg);
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

/// Positions of every group and node relative to their parent group.
#[derive(Debug, Clone)]
pub struct LocalLayout {
    pub group_local_pos: HashMap<GroupId, PointI>,
    pub node_local_pos: HashMap<NodeId, PointI>,
    /// Group boxes (including padding) relative to the group's own position.
    pub group_local_bounds: HashMap<GroupId, RectI>,
}

/// Lay out a diagram's groups bottom-up, then accumulate world coordinates.
///
/// `place_children(gid, group_local_pos, node_local_pos, group_local_bounds)` positions the
/// direct children of one group in its local coordinates; the bounds of its child groups
/// are already known when it is called.
pub fn layout_nested<F>(diagram: &Diagram, cfg: &LayoutConfig, place_children: F) -> LayoutResult
where
    F: FnMut(GroupId, &mut HashMap<GroupId, PointI>, &mut HashMap<NodeId, PointI>, &HashMap<GroupId, RectI>),
{
    to_world(diagram, cfg, &layout_nested_local(diagram, cfg, place_children))
}

/// The bottom-up half of [`layout_nested`]: local positions only.
pub fn layout_nested_local<F>(diagram: &Diagram, cfg: &LayoutConfig, mut place_children: F) -> LocalLayout
where
    F: FnMut(GroupId, &mut HashMap<GroupId, PointI>, &mut HashMap<NodeId, PointI>, &HashMap<GroupId, RectI>),
{
    let mut group_local_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut node_local_pos: HashMap<NodeId, PointI> = HashMap::new();
    let mut group_local_bounds: HashMap<GroupId, RectI> = HashMap::new();
    group_local_pos.insert(diagram.root, PointI { x: 0, y: 0 });

    // Children before parents, so a group's size is known before it gets placed.
    for gid in post_order_groups(diagram) {
        if gid != diagram.root {
            // Respect a constrained position; otherwise the parent will place it.
            let p = diagram.groups[gid.0].pos.unwrap_or(PointI { x: 0, y: 0 });
            group_local_pos.insert(gid, p);
        }
        place_children(gid, &mut group_local_pos, &mut node_local_pos, &group_local_bounds);
        let bounds = compute_group_local_bounds(diagram, gid, cfg, &group_local_pos, &node_local_pos, &group_local_bounds);
        group_local_bounds.insert(gid, bounds);
    }
    LocalLayout { group_local_pos, node_local_pos, group_local_bounds }
}

/// Accumulate local positions into world positions and bounds.
pub fn to_world(diagram: &Diagram, cfg: &LayoutConfig, local: &LocalLayout) -> LayoutResult {
    let LocalLayout { group_local_pos, node_local_pos, group_local_bounds } = local;
    let mut group_world_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut node_world_pos: HashMap<NodeId, PointI> = HashMap::new();
    let mut group_world_bounds: HashMap<GroupId, RectI> = HashMap::new();
    let mut node_world_bounds: HashMap<NodeId, RectI> = HashMap::new();

    // Parents before children, so a parent's world position is known.
    for gid in pre_order_groups(diagram) {
        let g_local = *group_local_pos.get(&gid).unwrap_or(&PointI { x: 0, y: 0 });
        let g_world = match diagram.groups[gid.0].parent {
            None => PointI { x: 0, y: 0 },
            Some(parent) => {
                let pw = group_world_pos[&parent];
                PointI { x: pw.x + g_local.x, y: pw.y + g_local.y }
            }
        };
        group_world_pos.insert(gid, g_world);

        let lb = group_local_bounds[&gid];
        group_world_bounds.insert(gid, RectI { x: g_world.x + lb.x, y: g_world.y + lb.y, w: lb.w, h: lb.h });

        for &nid in &diagram.groups[gid.0].children_nodes {
            let n_local = *node_local_pos.get(&nid).unwrap_or(&PointI { x: 0, y: 0 });
            let n_world = PointI { x: g_world.x + n_local.x, y: g_world.y + n_local.y };
            node_world_pos.insert(nid, n_world);
            let sz = get_node_size(&diagram.nodes[nid.0], cfg);
            node_world_bounds.insert(nid, RectI { x: n_world.x, y: n_world.y, w: sz.w, h: sz.h });
        }
    }

    LayoutResult {
        group_local_pos: group_local_pos.clone(),
        node_local_pos: node_local_pos.clone(),
        group_world_pos,
        node_world_pos,
        group_world_bounds,
        node_world_bounds,
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
