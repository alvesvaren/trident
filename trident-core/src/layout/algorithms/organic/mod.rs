//! Organic layout: stress-based placement with crossing reduction.
//!
//! Meant for graphs without a natural top-down flow, such as ER diagrams: connected
//! boxes end up near each other, edges rarely cross or run through boxes, and leaf
//! nodes (attributes) fan out around the box they belong to. The result only depends
//! on the diagram, so the same input always gives the same layout.
//!
//! Each group is laid out on its own, children first. Inside a group, its child
//! groups are rigid boxes and edges to nodes nested deeper are attached to the child
//! group that contains them. Locked (`@pos`) boxes stay where they are and the rest
//! is arranged around them.
//!
//! A group is laid out before anyone knows where its outside connections lead, so
//! afterwards each group is mirrored horizontally and/or vertically when that makes
//! the edges leaving it shorter and less tangled.

mod engine;
mod geom;

use std::collections::HashMap;

use crate::layout::{get_node_size, layout_nested_local, pre_order_groups, to_world, LayoutConfig, LayoutResult, LayoutStrategy, LocalLayout, RectI};
use crate::parser::{get_base_arrow_name, Diagram, GroupId, NodeId, PointI};
use engine::{Input, Params};
use geom::V2;

pub struct OrganicLayout;

impl LayoutStrategy for OrganicLayout {
    fn layout(&self, diagram: &Diagram, cfg: &LayoutConfig) -> LayoutResult {
        layout_organic(diagram, cfg)
    }
}

pub fn layout_organic(diagram: &Diagram, cfg: &LayoutConfig) -> LayoutResult {
    let mut local = layout_nested_local(diagram, cfg, |gid, group_local_pos, node_local_pos, group_local_bounds| {
        place_children(diagram, cfg, gid, group_local_pos, node_local_pos, group_local_bounds);
    });
    if diagram.groups.len() > 1 {
        mirror_groups(diagram, cfg, &mut local);
    }
    to_world(diagram, cfg, &local)
}

/// One crossing is worth this much extra edge length.
const CROSSING_LENGTH: f64 = 300.0;

/// Mirror each group (top-down) in whichever of the four ways makes the diagram's
/// edges least tangled, and the edges leaving the group shortest.
fn mirror_groups(diagram: &Diagram, cfg: &LayoutConfig, local: &mut LocalLayout) {
    for gid in pre_order_groups(diagram) {
        let group = &diagram.groups[gid.0];
        let locked = group.children_nodes.iter().any(|n| diagram.nodes[n.0].pos.is_some())
            || group.children_groups.iter().any(|g| diagram.groups[g.0].pos.is_some());
        if gid == diagram.root || locked || group.children_nodes.len() + group.children_groups.len() < 2 {
            continue;
        }
        let inside: Vec<bool> = diagram.nodes.iter().map(|n| is_within(diagram, n.group, gid)).collect();

        let original = local.clone();
        let mut best = (edge_cost(diagram, cfg, local, &inside), (false, false));
        for flip in [(true, false), (false, true), (true, true)] {
            *local = original.clone();
            mirror_children(diagram, cfg, local, gid, flip);
            let cost = edge_cost(diagram, cfg, local, &inside);
            if cost < best.0 - 1e-6 {
                best = (cost, flip);
            }
        }
        *local = original;
        mirror_children(diagram, cfg, local, gid, best.1);
    }
}

fn is_within(diagram: &Diagram, mut g: GroupId, ancestor: GroupId) -> bool {
    loop {
        if g == ancestor {
            return true;
        }
        match diagram.groups[g.0].parent {
            Some(p) => g = p,
            None => return false,
        }
    }
}

/// Mirror the direct children of `gid` inside the group's box; the box itself stays put.
fn mirror_children(diagram: &Diagram, cfg: &LayoutConfig, local: &mut LocalLayout, gid: GroupId, (flip_x, flip_y): (bool, bool)) {
    if !flip_x && !flip_y {
        return;
    }
    let b = local.group_local_bounds[&gid];
    let mirror = |p: PointI, offset: PointI, w: i32, h: i32| PointI {
        x: if flip_x { 2 * b.x + b.w - (p.x + offset.x) - w - offset.x } else { p.x },
        y: if flip_y { 2 * b.y + b.h - (p.y + offset.y) - h - offset.y } else { p.y },
    };
    let group = &diagram.groups[gid.0];
    for &n in &group.children_nodes {
        let s = get_node_size(&diagram.nodes[n.0], cfg);
        let p = local.node_local_pos[&n];
        local.node_local_pos.insert(n, mirror(p, PointI { x: 0, y: 0 }, s.w, s.h));
    }
    for &g in &group.children_groups {
        let cb = local.group_local_bounds[&g];
        let p = local.group_local_pos[&g];
        local.group_local_pos.insert(g, mirror(p, PointI { x: cb.x, y: cb.y }, cb.w, cb.h));
    }
}

/// Crossings among all edges, plus the length of the edges that leave the `inside` set.
fn edge_cost(diagram: &Diagram, cfg: &LayoutConfig, local: &LocalLayout, inside: &[bool]) -> f64 {
    let world = to_world(diagram, cfg, local);
    let center = |n: NodeId| {
        let r = world.node_world_bounds[&n];
        V2::new(r.x as f64 + r.w as f64 / 2.0, r.y as f64 + r.h as f64 / 2.0)
    };
    let segments: Vec<(NodeId, NodeId, V2, V2)> = diagram.edges.iter().map(|e| (e.from, e.to, center(e.from), center(e.to))).collect();
    let mut cost = 0.0;
    for (i, &(a, b, pa, pb)) in segments.iter().enumerate() {
        if inside[a.0] != inside[b.0] {
            cost += (pb - pa).len();
        }
        for &(c, d, pc, pd) in &segments[i + 1..] {
            if a != c && a != d && b != c && b != d && geom::segments_cross(pa, pb, pc, pd) {
                cost += CROSSING_LENGTH;
            }
        }
    }
    cost
}

#[derive(Clone, Copy)]
enum Item {
    Node(NodeId),
    Group(GroupId),
}

/// Where an item's box sits relative to its position, and how big it is.
struct Footprint {
    offset: V2,
    size: V2,
}

fn place_children(
    diagram: &Diagram,
    cfg: &LayoutConfig,
    gid: GroupId,
    group_local_pos: &mut HashMap<GroupId, PointI>,
    node_local_pos: &mut HashMap<NodeId, PointI>,
    group_local_bounds: &HashMap<GroupId, RectI>,
) {
    let group = &diagram.groups[gid.0];
    let mut items: Vec<(usize, Item)> = group
        .children_groups
        .iter()
        .map(|&g| (diagram.groups[g.0].order, Item::Group(g)))
        .chain(group.children_nodes.iter().map(|&n| (diagram.nodes[n.0].order, Item::Node(n))))
        .collect();
    if items.is_empty() {
        return;
    }
    items.sort_by_key(|(order, _)| *order);
    let items: Vec<Item> = items.into_iter().map(|(_, item)| item).collect();

    let footprints: Vec<Footprint> = items
        .iter()
        .map(|item| match *item {
            Item::Node(n) => {
                let s = get_node_size(&diagram.nodes[n.0], cfg);
                Footprint { offset: V2::default(), size: V2::new(s.w as f64, s.h as f64) }
            }
            Item::Group(g) => {
                let b = group_local_bounds[&g];
                Footprint { offset: V2::new(b.x as f64, b.y as f64), size: V2::new(b.w as f64, b.h as f64) }
            }
        })
        .collect();
    let locked: Vec<Option<PointI>> = items
        .iter()
        .map(|item| match *item {
            Item::Node(n) => diagram.nodes[n.0].pos,
            Item::Group(g) => diagram.groups[g.0].pos,
        })
        .collect();
    let fixed: Vec<Option<V2>> = locked
        .iter()
        .zip(&footprints)
        .map(|(p, f)| p.map(|p| V2::new(p.x as f64, p.y as f64) + f.offset + f.size * 0.5))
        .collect();

    // Attach every edge endpoint to the item at this level that contains it.
    let mut item_of_node: HashMap<NodeId, usize> = HashMap::new();
    let mut item_of_group: HashMap<GroupId, usize> = HashMap::new();
    for (i, item) in items.iter().enumerate() {
        match *item {
            Item::Node(n) => item_of_node.insert(n, i),
            Item::Group(g) => item_of_group.insert(g, i),
        };
    }
    let lift = |n: NodeId| -> Option<usize> {
        if let Some(&i) = item_of_node.get(&n) {
            return Some(i);
        }
        let mut g = diagram.nodes[n.0].group;
        loop {
            if let Some(&i) = item_of_group.get(&g) {
                return Some(i);
            }
            g = diagram.groups[g.0].parent?;
        }
    };

    let mut edges = Vec::new();
    let mut downward = Vec::new();
    for edge in &diagram.edges {
        let (Some(a), Some(b)) = (lift(edge.from), lift(edge.to)) else { continue };
        if a == b {
            continue;
        }
        edges.push((a, b));
        // Inheritance reads best with the parent above: `A --|> B` and `B <|-- A` both make B the parent.
        if matches!(get_base_arrow_name(&edge.arrow), "extends" | "implements") {
            downward.push(if edge.arrow.ends_with("_left") { (a, b) } else { (b, a) });
        }
    }

    let sizes: Vec<(f64, f64)> = footprints.iter().map(|f| (f.size.x, f.size.y)).collect();
    let params = Params { gap: cfg.gap as f64 * 0.75, ..Params::default() };
    let input = Input { sizes: &sizes, fixed: &fixed, edges: &edges, downward: &downward, initial: None };

    if fixed.iter().all(Option::is_some) {
        for (i, item) in items.iter().enumerate() {
            match *item {
                Item::Node(n) => node_local_pos.insert(n, locked[i].unwrap()),
                Item::Group(g) => group_local_pos.insert(g, locked[i].unwrap()),
            };
        }
        return;
    }

    // The layout this group gets with nothing locked, starting at the group's padding
    // like the other layouts.
    let unlocked_fixed = vec![None; items.len()];
    let mut unlocked = engine::layout(&Input { fixed: &unlocked_fixed, ..input }, &params);
    let min = unlocked.iter().zip(&footprints).fold(V2::new(f64::INFINITY, f64::INFINITY), |m, (c, f)| {
        let top_left = *c - f.size * 0.5;
        V2::new(m.x.min(top_left.x), m.y.min(top_left.y))
    });
    let shift = V2::new(cfg.group_padding as f64, cfg.group_padding as f64) - min;
    unlocked.iter_mut().for_each(|c| *c = *c + shift);

    // With locked boxes, the others start from where they'd be unlocked: dragging one
    // box (which locks it) then only nudges its surroundings instead of reshuffling.
    let centers = if fixed.iter().any(Option::is_some) {
        engine::layout(&Input { initial: Some(&unlocked), ..input }, &params)
    } else {
        unlocked
    };

    for (i, item) in items.iter().enumerate() {
        let top_left = centers[i] - footprints[i].size * 0.5 - footprints[i].offset;
        let p = locked[i].unwrap_or(PointI { x: top_left.x.round() as i32, y: top_left.y.round() as i32 });
        match *item {
            Item::Node(n) => node_local_pos.insert(n, p),
            Item::Group(g) => group_local_pos.insert(g, p),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{compile, parse_file};

    const ER: &str = "
rectangle Student
rectangle Course
rectangle Program
diamond Takes
diamond Teaches
diamond In
rectangle Teacher
circle idnr
circle name
circle code
Student --- Takes
Takes --- Course
Teacher --- Teaches
Teaches --- Course
Student --- In
In --- Program
Program --- Course
idnr --- Student
name --- Student
code --- Course
";

    fn centers(src: &str) -> HashMap<String, V2> {
        let diagram = compile(&parse_file(src).unwrap()).unwrap();
        let result = layout_organic(&diagram, &LayoutConfig::default());
        diagram
            .nodes
            .iter()
            .map(|n| {
                let r = result.node_world_bounds[&n.nid];
                (n.id.0.clone(), V2::new(r.x as f64 + r.w as f64 / 2.0, r.y as f64 + r.h as f64 / 2.0))
            })
            .collect()
    }

    #[test]
    fn same_input_same_layout() {
        assert_eq!(centers(ER), centers(ER));
    }

    #[test]
    fn dragging_one_box_leaves_the_rest_mostly_in_place() {
        let before = centers(ER);
        let teacher = before["Teacher"];
        let size = get_node_size(&compile(&parse_file(ER).unwrap()).unwrap().nodes[6], &LayoutConfig::default());
        // What the editor does on drop: lock the dragged box at its new top-left corner.
        let (x, y) = (teacher.x - size.w as f64 / 2.0 + 60.0, teacher.y - size.h as f64 / 2.0 + 40.0);
        let dragged = ER.replace("rectangle Teacher", &format!("rectangle Teacher {{\n    @pos: ({}, {})\n}}", x.round(), y.round()));
        let after = centers(&dragged);

        // Teacher -- Teaches -- Course may adjust; everything else should barely move.
        for (id, p) in &before {
            if id == "Teacher" || id == "Teaches" {
                continue;
            }
            let moved = (after[id] - *p).len();
            assert!(moved < 25.0, "{id} moved {moved:.0}px");
        }
    }
}
