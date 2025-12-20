/**
 * Shared type definitions for the diagram system
 */

export interface Bounds {
    x: number;
    y: number;
    w: number;
    h: number;
}

export interface DiagramNode {
    id: string;
    label: string | null;
    body_lines: string[];
    bounds: Bounds;
    has_pos: boolean;
    parent_offset: { x: number; y: number };
}

export interface DiagramEdge {
    from: string;
    to: string;
    arrow: string;
    label: string | null;
}

export interface DiagramGroup {
    id: string;
    bounds: Bounds;
}

export interface DiagramOutput {
    groups?: DiagramGroup[];
    nodes?: DiagramNode[];
    edges?: DiagramEdge[];
    error?: string;
}

/** Drag state for tracking node/group dragging */
export interface DragState {
    type: "node" | "group";
    id: string;
    groupIndex?: number;
    startX: number;
    startY: number;
    startMouseX: number;
    startMouseY: number;
    parentOffsetX: number;
    parentOffsetY: number;
    currentX: number;
    currentY: number;
}
