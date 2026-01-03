//
// Compile step: FileAst (surface AST) -> Diagram (layout-friendly IR)
//
// What this does:
// - Creates a synthetic root group
// - Flattens nested GroupAst/NodeAst into indexed vectors with parent pointers
// - Enforces global uniqueness:
//     - node identifiers must be unique
//     - named group identifiers must be unique
// - Resolves RelationAst endpoints from Ident -> NodeId
// - Preserves deterministic order using the original traversal order
//
// Assumptions:
// - Uses the AST types from the parser code (Ident, PointI, etc.)
// - Layout can be implemented over Diagram directly.

use std::collections::HashMap;

use crate::parser::{FileAst, GroupAst, Ident, NodeAst, PointI, RelationAst, Stmt};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct Diagram {
    pub root: GroupId,
    pub groups: Vec<Group>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct GroupId(pub usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone, Serialize)]
pub struct Group {
    pub gid: GroupId,
    /// None => anonymous group
    pub id: Option<Ident>,
    pub parent: Option<GroupId>,
    pub pos: Option<PointI>, // local to parent
    pub children_groups: Vec<GroupId>,
    pub children_nodes: Vec<NodeId>,
    /// Stable traversal order index (assigned during compilation).
    pub order: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub nid: NodeId,
    /// Node kind: "class" or "node"
    pub kind: String,
    /// Modifiers: "abstract", "interface", "enum", "rectangle", "circle", "diamond", etc.
    pub modifiers: Vec<String>,
    /// Unique identifier
    pub id: Ident,
    pub label: Option<String>,
    pub group: GroupId,
    pub pos: Option<PointI>, // local to group
    /// Custom width (from @width directive)
    pub width: Option<i32>,
    /// Custom height (from @height directive)
    pub height: Option<i32>,
    pub body_lines: Vec<String>,
    /// Whether this node was explicitly declared (false for implicit nodes)
    pub explicit: bool,
    /// Stable traversal order index.
    pub order: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    /// Arrow canonical name (e.g., "extends_left", "assoc_right")
    pub arrow: String,
    pub label: Option<String>,
    /// Stable traversal order index.
    pub order: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompileError {
    pub msg: String,
    pub line: usize,  // 1-based line number
    pub col: usize,   // 1-based column (usually 1)
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Compile error at {}:{}: {}", self.line, self.col, self.msg)
    }
}
impl std::error::Error for CompileError {}

pub fn compile(ast: &FileAst) -> Result<Diagram, CompileError> {
    let mut ctx = CompileCtx::new();

    // Create synthetic root group (order 0)
    let root = ctx.new_group(None, None, None);

    // Walk file statements into root group
    ctx.compile_items_into_group(&ast.items, root)?;

    // Resolve edges after all nodes exist
    ctx.resolve_edges()?;

    Ok(ctx.finish())
}

struct PendingEdge {
    from: Ident,
    to: Ident,
    arrow: String,
    label: Option<String>,
    order: usize,
    line: usize,  // For error reporting
}

struct CompileCtx {
    groups: Vec<Group>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,

    // For uniqueness checks and resolving
    node_by_ident: HashMap<Ident, NodeId>,
    group_by_ident: HashMap<Ident, GroupId>,

    pending_edges: Vec<PendingEdge>,

    next_order: usize,
}

impl CompileCtx {
    fn new() -> Self {
        Self {
            groups: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            node_by_ident: HashMap::new(),
            group_by_ident: HashMap::new(),
            pending_edges: Vec::new(),
            next_order: 0,
        }
    }

    fn finish(self) -> Diagram {
        Diagram {
            root: GroupId(0),
            groups: self.groups,
            nodes: self.nodes,
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
            children_nodes: Vec::new(),
            order,
        });
        gid
    }

    fn new_node(
        &mut self,
        kind: String,
        modifiers: Vec<String>,
        id: Ident,
        label: Option<String>,
        group: GroupId,
        pos: Option<PointI>,
        width: Option<i32>,
        height: Option<i32>,
        body_lines: Vec<String>,
        explicit: bool,
    ) -> NodeId {
        let nid = NodeId(self.nodes.len());
        let order = self.alloc_order();
        self.nodes.push(Node {
            nid,
            kind,
            modifiers,
            id,
            label,
            group,
            pos,
            width,
            height,
            body_lines,
            explicit,
            order,
        });
        nid
    }

    fn compile_items_into_group(&mut self, items: &[Stmt], parent_gid: GroupId) -> Result<(), CompileError> {
        for stmt in items {
            match stmt {
                Stmt::Group(g) => self.compile_group(g, parent_gid)?,
                Stmt::Node(n) => self.compile_node(n, parent_gid)?,
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
                    line: g.span.map(|s| s.start_line).unwrap_or(1),
                    col: 1,
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

    fn compile_node(&mut self, n: &NodeAst, parent_gid: GroupId) -> Result<(), CompileError> {
        // Build modifiers: include original_kind if it differs from kind
        // (e.g., "enum" for class, "circle" for node)
        let mut modifiers = n.modifiers.clone();
        if n.original_kind != n.kind {
            modifiers.push(n.original_kind.clone());
        }
        
        // Check if node already exists (could be implicit from a relation)
        if let Some(&existing_nid) = self.node_by_ident.get(&n.id) {
            let existing = &mut self.nodes[existing_nid.0];
            if existing.explicit {
                // Already explicitly declared - duplicate error
                return Err(CompileError {
                    msg: format!("duplicate node identifier: {}", n.id.0),
                    line: n.span.map(|s| s.start_line).unwrap_or(1),
                    col: 1,
                });
            }
            
            // Upgrade implicit node to explicit
            existing.kind = n.kind.clone();
            existing.modifiers = modifiers;
            existing.label = n.label.clone();
            existing.width = n.width;
            existing.height = n.height;
            existing.body_lines = n.body_lines.clone();
            existing.explicit = true;
            if n.pos.is_some() {
                existing.pos = n.pos;
            }
            
            // Update group membership if needed
            if existing.group != parent_gid {
                // Remove from old group
                let old_group = &mut self.groups[existing.group.0];
                old_group.children_nodes.retain(|&nid| nid != existing_nid);
                // Add to new group
                existing.group = parent_gid;
                self.groups[parent_gid.0].children_nodes.push(existing_nid);
            }
            
            return Ok(());
        }

        let nid = self.new_node(
            n.kind.clone(),
            modifiers,
            n.id.clone(),
            n.label.clone(),
            parent_gid,
            n.pos,
            n.width,
            n.height,
            n.body_lines.clone(),
            true, // explicit
        );

        self.node_by_ident.insert(n.id.clone(), nid);

        // Link to group
        self.groups[parent_gid.0].children_nodes.push(nid);

        Ok(())
    }

    fn collect_relation(&mut self, r: &RelationAst) -> Result<(), CompileError> {
        let order = self.alloc_order();
        self.pending_edges.push(PendingEdge {
            from: r.from.clone(),
            to: r.to.clone(),
            arrow: r.arrow.clone(),
            label: r.label.clone(),
            order,
            line: r.span.map(|s| s.start_line).unwrap_or(1),
        });
        Ok(())
    }

    /// Get an existing node by identifier, or create an implicit one.
    fn get_or_create_implicit_node(&mut self, id: &Ident) -> NodeId {
        if let Some(&nid) = self.node_by_ident.get(id) {
            return nid;
        }
        
        // Create implicit node in root group (GroupId(0))
        let nid = self.new_node(
            "node".to_string(),
            vec!["rectangle".to_string()],
            id.clone(),
            None,
            GroupId(0),
            None,
            None,
            None,
            Vec::new(),
            false, // implicit
        );
        self.node_by_ident.insert(id.clone(), nid);
        self.groups[0].children_nodes.push(nid);
        nid
    }

    fn resolve_edges(&mut self) -> Result<(), CompileError> {
        // Collect pending edges (drain to avoid borrow issues)
        let pending: Vec<_> = self.pending_edges.drain(..).collect();
        
        for pe in pending {
            // Create implicit nodes if needed
            let from = self.get_or_create_implicit_node(&pe.from);
            let to = self.get_or_create_implicit_node(&pe.to);

            self.edges.push(Edge {
                from,
                to,
                arrow: pe.arrow,
                label: pe.label,
                order: pe.order,
            });
        }

        Ok(())
    }
}