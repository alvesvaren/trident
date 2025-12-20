// sdd_layout.rs
//
// Graph-driven deterministic layouter for Diagram.
//
// Goals:
// - Deterministic: no randomness, no time budgets
// - Manual + auto: any node/group with pos is fixed (local to parent)
// - Deepest-first: layout children, then treat child groups as boxes
// - Graph-driven placement: place connected nodes near each other
// - No overlap (in local coordinates)
// - Produces local positions for ALL groups/classes (fills in pos=None)
//
// Submodules:
// - spatial_grid: O(1) overlap detection
// - adjacency: edge weight computation
// - placement: graph-driven placement algorithm
//
// Output:
// - LayoutResult with world positions + sizes + group bboxes.

use std::collections::HashMap;

use crate::parser::{PointI, Diagram, GroupId, ClassId};
use serde::Serialize;

mod spatial_grid;
mod adjacency;
mod placement;

use adjacency::Adjacency;
use placement::layout_group_children_graph_driven;


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
    /// Size for classes (v0.0.1: constant).
    pub class_size: SizeI,
    /// Minimum size for groups (even if empty).
    pub min_group_size: SizeI,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            group_padding: 24,
            gap: 24,
            max_row_w: 1200,
            class_size: SizeI { w: 220, h: 120 },
            min_group_size: SizeI { w: 200, h: 120 },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LayoutResult {
    /// Local positions (relative to parent group) for all groups/classes.
    pub group_local_pos: HashMap<GroupId, PointI>,
    pub class_local_pos: HashMap<ClassId, PointI>,

    /// World positions for all groups/classes (after accumulation).
    pub group_world_pos: HashMap<GroupId, PointI>,
    pub class_world_pos: HashMap<ClassId, PointI>,

    /// Group bounds in world coordinates (including padding).
    pub group_world_bounds: HashMap<GroupId, RectI>,

    /// Class bounds in world coordinates.
    pub class_world_bounds: HashMap<ClassId, RectI>,
}

pub fn layout_diagram(diagram: &Diagram, cfg: &LayoutConfig) -> LayoutResult {
    // We'll fill these in for all nodes. Root is fixed at (0,0) local/world.
    let mut group_local_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut class_local_pos: HashMap<ClassId, PointI> = HashMap::new();

    group_local_pos.insert(diagram.root, PointI { x: 0, y: 0 });

    // Build adjacency from edges for graph-driven placement
    let adjacency = Adjacency::from_diagram(diagram);

    // Layout groups bottom-up (children first). Our compiler creates parents before children,
    // but for layout we want post-order traversal.
    let post = post_order_groups(diagram);

    // We also need per-group child sizes during packing.
    let mut group_local_bounds: HashMap<GroupId, RectI> = HashMap::new();

    // First pass: compute local positions for everything, and local bounds for groups.
    for gid in post {
        let g = &diagram.groups[gid.0];

        // Determine this group's own local position:
        // - If constrained (pos present), respect it.
        // - Else if root, it's already (0,0).
        // - Else default (0,0) for now (parent will place it).
        if gid != diagram.root {
            let p = g.pos.unwrap_or(PointI { x: 0, y: 0 });
            group_local_pos.insert(gid, p);
        }

        // Lay out children within this group using graph-driven placement.
        // Connected nodes will be placed closer together.
        layout_group_children_graph_driven(
            diagram,
            gid,
            cfg,
            &adjacency,
            &mut group_local_pos,
            &mut class_local_pos,
            &group_local_bounds, // contains bounds for child groups already
        );

        // After children placed, compute this group's local bounds (container box).
        let bounds = compute_group_local_bounds(
            diagram,
            gid,
            cfg,
            &group_local_pos,
            &class_local_pos,
            &group_local_bounds,
        );
        group_local_bounds.insert(gid, bounds);
    }

    // Second pass: accumulate world positions and compute world bounds.
    let mut group_world_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut class_world_pos: HashMap<ClassId, PointI> = HashMap::new();
    let mut group_world_bounds: HashMap<GroupId, RectI> = HashMap::new();
    let mut class_world_bounds: HashMap<ClassId, RectI> = HashMap::new();

    group_world_pos.insert(diagram.root, PointI { x: 0, y: 0 });

    // Traverse groups in pre-order so parents have world pos before children.
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

        // Convert local bounds -> world bounds
        let lb = *group_local_bounds.get(&gid).unwrap();
        let wb = RectI { x: g_world.x + lb.x, y: g_world.y + lb.y, w: lb.w, h: lb.h };
        group_world_bounds.insert(gid, wb);

        // Classes directly in this group
        for &cid in &diagram.groups[gid.0].children_classes {
            let c_local = *class_local_pos.get(&cid).unwrap_or(&PointI { x: 0, y: 0 });
            let c_world = PointI { x: g_world.x + c_local.x, y: g_world.y + c_local.y };
            class_world_pos.insert(cid, c_world);

            let sz = cfg.class_size;
            class_world_bounds.insert(cid, RectI { x: c_world.x, y: c_world.y, w: sz.w, h: sz.h });
        }
    }

    LayoutResult {
        group_local_pos,
        class_local_pos,
        group_world_pos,
        class_world_pos,
        group_world_bounds,
        class_world_bounds,
    }
}

/// Compute a group's container bounds in LOCAL coordinates, based on children.
/// This includes padding. If group has no children, returns min_group_size at (0,0).
fn compute_group_local_bounds(
    diagram: &Diagram,
    gid: GroupId,
    cfg: &LayoutConfig,
    group_local_pos: &HashMap<GroupId, PointI>,
    class_local_pos: &HashMap<ClassId, PointI>,
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

    // include child classes
    for &cid in &g.children_classes {
        let p = *class_local_pos.get(&cid).unwrap_or(&PointI { x: 0, y: 0 });
        let sz = cfg.class_size;
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
fn post_order_groups(diagram: &Diagram) -> Vec<GroupId> {
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
fn pre_order_groups(diagram: &Diagram) -> Vec<GroupId> {
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
