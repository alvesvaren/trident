//
// Compile step: FileAst (surface AST) -> Diagram (layout-friendly IR)
//
// What this does:
// - Creates a synthetic root group
// - Flattens nested GroupAst/ClassAst into indexed vectors with parent pointers
// - Enforces global uniqueness:
//     - class identifiers must be unique
//     - named group identifiers must be unique
// - Resolves RelationAst endpoints from Ident -> ClassId
// - Preserves deterministic order using the original traversal order
//
// Assumptions:
// - Uses the AST types from the parser code (Ident, PointI, Arrow, etc.)
// - Layout can be implemented over Diagram directly.

use std::collections::HashMap;

use crate::parser::{Arrow, ClassAst, FileAst, GroupAst, Ident, PointI, RelationAst, Stmt};
use serde::{Serialize};

#[derive(Debug, Clone)]
pub struct Diagram {
    pub root: GroupId,
    pub groups: Vec<Group>,
    pub classes: Vec<Class>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct GroupId(pub usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ClassId(pub usize);

#[derive(Debug, Clone, Serialize)]
pub struct Group {
    pub gid: GroupId,
    /// None => anonymous group
    pub id: Option<Ident>,
    pub parent: Option<GroupId>,
    pub pos: Option<PointI>, // local to parent
    pub children_groups: Vec<GroupId>,
    pub children_classes: Vec<ClassId>,
    /// Stable traversal order index (assigned during compilation).
    pub order: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Class {
    pub cid: ClassId,
    pub id: Ident,
    pub label: Option<String>,
    pub group: GroupId,
    pub pos: Option<PointI>, // local to group
    pub body_lines: Vec<String>,
    /// Stable traversal order index.
    pub order: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Edge {
    pub from: ClassId,
    pub to: ClassId,
    pub arrow: Arrow,
    pub label: Option<String>,
    /// Stable traversal order index.
    pub order: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompileError {
    pub msg: String,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Compile error: {}", self.msg)
    }
}
impl std::error::Error for CompileError {}

pub fn compile(ast: &FileAst) -> Result<Diagram, CompileError> {
    let mut ctx = CompileCtx::new();

    // Create synthetic root group (order 0)
    let root = ctx.new_group(None, None, None);

    // Walk file statements into root group
    ctx.compile_items_into_group(&ast.items, root)?;

    // Resolve edges after all classes exist
    ctx.resolve_edges()?;

    Ok(ctx.finish())
}

struct PendingEdge {
    from: Ident,
    to: Ident,
    arrow: Arrow,
    label: Option<String>,
    order: usize,
}

struct CompileCtx {
    groups: Vec<Group>,
    classes: Vec<Class>,
    edges: Vec<Edge>,

    // For uniqueness checks and resolving
    class_by_ident: HashMap<Ident, ClassId>,
    group_by_ident: HashMap<Ident, GroupId>,

    pending_edges: Vec<PendingEdge>,

    next_order: usize,
}

impl CompileCtx {
    fn new() -> Self {
        Self {
            groups: Vec::new(),
            classes: Vec::new(),
            edges: Vec::new(),
            class_by_ident: HashMap::new(),
            group_by_ident: HashMap::new(),
            pending_edges: Vec::new(),
            next_order: 0,
        }
    }

    fn finish(self) -> Diagram {
        Diagram {
            root: GroupId(0),
            groups: self.groups,
            classes: self.classes,
            edges: self.edges,
        }
    }

    fn alloc_order(&mut self) -> usize {
        let o = self.next_order;
        self.next_order += 1;
        o
    }

    fn new_group(
        &mut self,
        id: Option<Ident>,
        parent: Option<GroupId>,
        pos: Option<PointI>,
    ) -> GroupId {
        let gid = GroupId(self.groups.len());
        let order = self.alloc_order();
        self.groups.push(Group {
            gid,
            id,
            parent,
            pos,
            children_groups: Vec::new(),
            children_classes: Vec::new(),
            order,
        });
        gid
    }

    fn new_class(
        &mut self,
        id: Ident,
        label: Option<String>,
        group: GroupId,
        pos: Option<PointI>,
        body_lines: Vec<String>,
    ) -> ClassId {
        let cid = ClassId(self.classes.len());
        let order = self.alloc_order();
        self.classes.push(Class {
            cid,
            id,
            label,
            group,
            pos,
            body_lines,
            order,
        });
        cid
    }

    fn compile_items_into_group(&mut self, items: &[Stmt], parent_gid: GroupId) -> Result<(), CompileError> {
        for stmt in items {
            match stmt {
                Stmt::Group(g) => self.compile_group(g, parent_gid)?,
                Stmt::Class(c) => self.compile_class(c, parent_gid)?,
                Stmt::Relation(r) => self.collect_relation(r)?,
                Stmt::Comment(_) => {} // Comments don't affect the diagram
            }
        }
        Ok(())
    }

    fn compile_group(&mut self, g: &GroupAst, parent_gid: GroupId) -> Result<(), CompileError> {
        // Uniqueness check for named groups
        if let Some(id) = &g.id {
            if self.group_by_ident.contains_key(id) {
                return Err(CompileError {
                    msg: format!("duplicate group identifier: {}", id.0),
                });
            }
        }

        let gid = self.new_group(g.id.clone(), Some(parent_gid), g.pos);

        // Register group id if named
        if let Some(id) = &self.groups[gid.0].id {
            self.group_by_ident.insert(id.clone(), gid);
        }

        // Link to parent
        self.groups[parent_gid.0].children_groups.push(gid);

        // Recurse into children
        self.compile_items_into_group(&g.items, gid)?;

        Ok(())
    }

    fn compile_class(&mut self, c: &ClassAst, parent_gid: GroupId) -> Result<(), CompileError> {
        // Uniqueness check for classes
        if self.class_by_ident.contains_key(&c.id) {
            return Err(CompileError {
                msg: format!("duplicate class identifier: {}", c.id.0),
            });
        }

        let cid = self.new_class(
            c.id.clone(),
            c.label.clone(),
            parent_gid,
            c.pos,
            c.body_lines.clone(),
        );

        self.class_by_ident.insert(c.id.clone(), cid);

        // Link to group
        self.groups[parent_gid.0].children_classes.push(cid);

        Ok(())
    }

    fn collect_relation(&mut self, r: &RelationAst) -> Result<(), CompileError> {
        let order = self.alloc_order();
        self.pending_edges.push(PendingEdge {
            from: r.from.clone(),
            to: r.to.clone(),
            arrow: r.arrow,
            label: r.label.clone(),
            order,
        });
        Ok(())
    }

    fn resolve_edges(&mut self) -> Result<(), CompileError> {
        for pe in self.pending_edges.drain(..) {
            let from = self.class_by_ident.get(&pe.from).copied().ok_or_else(|| CompileError {
                msg: format!("edge references unknown class '{}'", pe.from.0),
            })?;
            let to = self.class_by_ident.get(&pe.to).copied().ok_or_else(|| CompileError {
                msg: format!("edge references unknown class '{}'", pe.to.0),
            })?;

            self.edges.push(Edge {
                from,
                to,
                arrow: pe.arrow,
                label: pe.label,
                order: pe.order,
            });
        }

        // Keep edges deterministic: they are already in compilation order
        Ok(())
    }
}

// ---------- Optional helpers for layout ----------

/// Compute world positions by accumulating parent group offsets.
/// Auto-placed items will have pos == None; layout should fill those in first.
/// This helper just accumulates existing constraints.
///
/// Returns a map for groups and classes containing computed world positions for those with known pos.
/// If a group/class (or any ancestor group) has missing pos, it is omitted from output.
pub fn compute_world_positions(d: &Diagram) -> (HashMap<GroupId, PointI>, HashMap<ClassId, PointI>) {
    let mut gw: HashMap<GroupId, PointI> = HashMap::new();
    let mut cw: HashMap<ClassId, PointI> = HashMap::new();

    // Root is world (0,0)
    gw.insert(d.root, PointI { x: 0, y: 0 });

    // Traverse groups in creation order (parents always created before children in this compiler)
    for g in &d.groups {
        let gid = g.gid;
        if gid == d.root {
            continue;
        }

        let parent = match g.parent {
            Some(p) => p,
            None => continue,
        };

        let parent_w = match gw.get(&parent).copied() {
            Some(p) => p,
            None => continue,
        };

        let local = match g.pos {
            Some(p) => p,
            None => continue,
        };

        gw.insert(
            gid,
            PointI {
                x: parent_w.x + local.x,
                y: parent_w.y + local.y,
            },
        );
    }

    // Classes
    for c in &d.classes {
        let group_w = match gw.get(&c.group).copied() {
            Some(p) => p,
            None => continue,
        };
        let local = match c.pos {
            Some(p) => p,
            None => continue,
        };

        cw.insert(
            c.cid,
            PointI {
                x: group_w.x + local.x,
                y: group_w.y + local.y,
            },
        );
    }

    (gw, cw)
}
