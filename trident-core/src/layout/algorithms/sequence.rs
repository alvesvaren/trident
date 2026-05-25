//! Sequence diagram layout algorithm.
//!
//! Places participants horizontally at the top with vertical lifelines,
//! and messages as horizontal arrows at increasing Y positions.

use std::collections::HashMap;

use crate::layout::{LayoutConfig, LayoutResult, RectI, SizeI};
use crate::parser::{Diagram, DiagramKind, GroupId, NodeId, PointI};

const PARTICIPANT_MIN_WIDTH: i32 = 80;
const PARTICIPANT_HEIGHT: i32 = 36;
const PARTICIPANT_GAP: i32 = 60;
const MESSAGE_ROW_HEIGHT: i32 = 48;
const LIFELINE_TOP_GAP: i32 = 16;
const LIFELINE_BOTTOM_PADDING: i32 = 40;
const ACTIVATION_WIDTH: i32 = 14;

#[derive(Debug, Clone)]
pub struct SequenceLayoutData {
    pub lifelines: Vec<SequenceLifeline>,
    pub activations: Vec<SequenceActivation>,
    pub message_y: HashMap<usize, i32>,
}

#[derive(Debug, Clone)]
pub struct SequenceLifeline {
    pub participant: NodeId,
    pub x: i32,
    pub y1: i32,
    pub y2: i32,
}

#[derive(Debug, Clone)]
pub struct SequenceActivation {
    pub participant: NodeId,
    pub x: i32,
    pub y: i32,
    pub h: i32,
}

pub fn layout_sequence(diagram: &Diagram, cfg: &LayoutConfig) -> (LayoutResult, SequenceLayoutData) {
    assert_eq!(diagram.kind, DiagramKind::Sequence);

    let mut node_local_pos: HashMap<NodeId, PointI> = HashMap::new();
    let mut node_local_bounds: HashMap<NodeId, RectI> = HashMap::new();
    let mut group_local_pos: HashMap<GroupId, PointI> = HashMap::new();
    let mut group_local_bounds: HashMap<GroupId, RectI> = HashMap::new();

    group_local_pos.insert(diagram.root, PointI { x: 0, y: 0 });

    // Participants in declaration order
    let mut participants: Vec<NodeId> = diagram.nodes.iter().map(|n| n.nid).collect();
    participants.sort_by_key(|nid| diagram.nodes[nid.0].order);

    // Compute participant widths
    let mut participant_sizes: HashMap<NodeId, SizeI> = HashMap::new();
    for &nid in &participants {
        let node = &diagram.nodes[nid.0];
        let sz = participant_size(node, cfg);
        participant_sizes.insert(nid, sz);
    }

    // Place participants horizontally
    let mut x = 0i32;
    for &nid in &participants {
        let sz = participant_sizes[&nid];
        node_local_pos.insert(nid, PointI { x, y: 0 });
        node_local_bounds.insert(nid, RectI { x, y: 0, w: sz.w, h: sz.h });
        x += sz.w + PARTICIPANT_GAP;
    }

    let lifeline_top = PARTICIPANT_HEIGHT + LIFELINE_TOP_GAP;

    // Sort edges by order to get message sequence
    let mut message_edges: Vec<(usize, &crate::parser::compile::Edge)> = diagram
        .edges
        .iter()
        .enumerate()
        .collect();
    message_edges.sort_by_key(|(_, e)| e.order);

    let message_count = message_edges.len();
    let lifeline_bottom = if message_count == 0 {
        lifeline_top + LIFELINE_BOTTOM_PADDING
    } else {
        lifeline_top + (message_count as i32) * MESSAGE_ROW_HEIGHT + LIFELINE_BOTTOM_PADDING
    };

    let mut message_y: HashMap<usize, i32> = HashMap::new();
    for (msg_idx, (edge_idx, _)) in message_edges.iter().enumerate() {
        let y = lifeline_top + (msg_idx as i32) * MESSAGE_ROW_HEIGHT + MESSAGE_ROW_HEIGHT / 2;
        message_y.insert(*edge_idx, y);
    }

    // Build lifelines at participant centers
    let mut lifelines = Vec::new();
    for &nid in &participants {
        let pos = node_local_pos[&nid];
        let sz = participant_sizes[&nid];
        let center_x = pos.x + sz.w / 2;
        lifelines.push(SequenceLifeline {
            participant: nid,
            x: center_x,
            y1: pos.y + sz.h,
            y2: lifeline_bottom,
        });
    }

    // Build activation boxes
    let mut activations = Vec::new();
    for span in &diagram.activations {
        let pos = node_local_pos.get(&span.participant);
        let lifeline = lifelines.iter().find(|l| l.participant == span.participant);
        if pos.is_none() || lifeline.is_none() {
            continue;
        }

        let start_y = lifeline_top
            + (span.start_message as i32) * MESSAGE_ROW_HEIGHT
            + MESSAGE_ROW_HEIGHT / 4;
        let end_y = lifeline_top
            + (span.end_message as i32) * MESSAGE_ROW_HEIGHT
            + (MESSAGE_ROW_HEIGHT * 3) / 4;

        activations.push(SequenceActivation {
            participant: span.participant,
            x: lifeline.unwrap().x - ACTIVATION_WIDTH / 2,
            y: start_y,
            h: (end_y - start_y).max(MESSAGE_ROW_HEIGHT / 2),
        });
    }

    // World positions (flat diagram, no nested groups)
    let mut node_world_pos = HashMap::new();
    let mut node_world_bounds = HashMap::new();
    for &nid in &participants {
        let local = node_local_pos[&nid];
        node_world_pos.insert(nid, local);
        node_world_bounds.insert(nid, node_local_bounds[&nid]);
    }

    let group_world_pos = group_local_pos.clone();

    let layout = LayoutResult {
        group_local_pos,
        node_local_pos,
        group_world_pos,
        node_world_pos,
        group_world_bounds: group_local_bounds,
        node_world_bounds,
    };

    let seq_data = SequenceLayoutData {
        lifelines,
        activations,
        message_y,
    };

    (layout, seq_data)
}

fn participant_size(node: &crate::parser::compile::Node, cfg: &LayoutConfig) -> SizeI {
    if let (Some(w), Some(h)) = (node.width, node.height) {
        return SizeI { w, h };
    }

    let text = node.label.as_ref().unwrap_or(&node.id.0);
    let text_width = text.chars().count() as i32 * cfg.node_rendering.char_width;
    let w = (text_width + 2 * cfg.node_rendering.padding).max(PARTICIPANT_MIN_WIDTH);
    SizeI { w, h: PARTICIPANT_HEIGHT }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::compile::{Diagram, Edge, Node, Group};
    use crate::parser::{Ident, GroupId, NodeId, DiagramKind};

    fn make_sequence_diagram() -> Diagram {
        Diagram {
            kind: DiagramKind::Sequence,
            root: GroupId(0),
            groups: vec![Group {
                gid: GroupId(0),
                id: None,
                parent: None,
                pos: None,
                children_groups: vec![],
                children_nodes: vec![NodeId(0), NodeId(1)],
                order: 0,
            }],
            nodes: vec![
                Node {
                    nid: NodeId(0),
                    kind: "participant".into(),
                    modifiers: vec![],
                    id: Ident("Alice".into()),
                    label: None,
                    group: GroupId(0),
                    pos: None,
                    width: None,
                    height: None,
                    body_lines: vec![],
                    explicit: true,
                    order: 1,
                },
                Node {
                    nid: NodeId(1),
                    kind: "participant".into(),
                    modifiers: vec![],
                    id: Ident("Bob".into()),
                    label: None,
                    group: GroupId(0),
                    pos: None,
                    width: None,
                    height: None,
                    body_lines: vec![],
                    explicit: true,
                    order: 2,
                },
            ],
            edges: vec![Edge {
                from: NodeId(0),
                to: NodeId(1),
                arrow: "seq_message".into(),
                label: Some("Hello".into()),
                order: 3,
            }],
            activations: vec![],
        }
    }

    #[test]
    fn test_sequence_layout_participants_horizontal() {
        let diagram = make_sequence_diagram();
        let (layout, _seq) = layout_sequence(&diagram, &LayoutConfig::default());

        let alice = layout.node_world_bounds.get(&NodeId(0)).unwrap();
        let bob = layout.node_world_bounds.get(&NodeId(1)).unwrap();

        assert!(bob.x > alice.x);
        assert_eq!(alice.y, 0);
        assert_eq!(bob.y, 0);
    }

    #[test]
    fn test_sequence_layout_message_y() {
        let diagram = make_sequence_diagram();
        let (_layout, seq) = layout_sequence(&diagram, &LayoutConfig::default());

        assert!(seq.message_y.contains_key(&0));
        let y = seq.message_y[&0];
        assert!(y > PARTICIPANT_HEIGHT);
    }
}
